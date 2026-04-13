//! WebAssembly bindings for the `contrato` contract system.
//!
//! Exposes [`contrato::Contract`] and [`contrato::Universe`] to
//! JavaScript as `Contract` and `Universe` via `wasm-bindgen`. The
//! surface covers construction, accessors, children mutation, matcher
//! search, requirement validation, and the read-only requirement-index
//! accessors needed to compose cross-reference walks on the JS side.
//!
//! Matchers cross the boundary as plain JS objects (e.g.
//! `{ type: 'sw.os', slug: 'debian' }`), deserialized into
//! [`contrato::ContractMatcher`] at each call site. A typed wrapper can
//! be introduced later if the looser typing becomes a problem — for
//! now a plain object is the minimum-overhead path.
//!
//! Each wrapper owns its underlying `contrato` value by clone. Methods
//! that return child contracts clone the matched values into fresh
//! `WasmContract` handles — cheap because `contrato::Contract` only
//! clones owned index state (the lazy hash cell is preserved by clone).
//! Methods that mutate the internal search cache are exposed as
//! `&mut self` on the JS side; wasm-bindgen enforces borrow discipline
//! per call.

use contrato::{Contract, ContractMatcher, RawContract};
use js_sys::Array;
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;

// ── shared helpers ────────────────────────────────────────────────────────

/// Returns a JSON-compatible serde-wasm-bindgen serializer.
///
/// Emits plain JS objects for maps (rather than `Map` instances) so
/// the JS side can use the usual dot / bracket access on anything
/// returned from `toJSON`, requirement payloads, or serialized
/// matchers without an adapter step.
fn json_serializer() -> Serializer {
    Serializer::new().serialize_maps_as_objects(true)
}

/// Converts any displayable error into a `JsValue` suitable for
/// `Result::Err` on a `#[wasm_bindgen]` method.
fn wasm_err(e: impl std::fmt::Display) -> JsValue {
    JsError::new(&e.to_string()).into()
}

/// Wraps a collection of owned `contrato::Contract` values into a JS
/// array of `WasmContract` handles.
fn contracts_to_array<I>(contracts: I) -> Array
where
    I: IntoIterator<Item = Contract>,
{
    let arr = Array::new();
    for c in contracts {
        arr.push(&JsValue::from(WasmContract { inner: c }));
    }
    arr
}

/// Converts an optional `Vec<String>` filter into an owned
/// `Vec<&str>` usable as `Option<&[&str]>` on the Rust API.
///
/// Returns `None` when the caller passed `undefined`; otherwise
/// returns `Some(vec_of_str)`. The returned `Vec` keeps the borrowed
/// string slices alive for the caller, which then builds the
/// `&[&str]` via `.as_deref()` on an `Option<Vec<&str>>`.
fn borrow_type_filter(types: &Option<Vec<String>>) -> Option<Vec<&str>> {
    types
        .as_ref()
        .map(|v| v.iter().map(String::as_str).collect())
}

/// Deserializes a [`ContractMatcher`] from a plain JS value.
///
/// Shared by every `WasmContract` method that accepts a matcher so the
/// deserialization error shape stays consistent regardless of whether
/// the caller is searching, capability-matching, or doing anything
/// else that takes a matcher argument.
fn matcher_from_js(value: JsValue) -> Result<ContractMatcher, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(wasm_err)
}

// ── Contract ──────────────────────────────────────────────────────────────

/// JavaScript wrapper around [`contrato::Contract`].
///
/// Exposed to JS as the class `Contract`.
#[wasm_bindgen(js_name = Contract)]
pub struct WasmContract {
    inner: Contract,
}

#[wasm_bindgen(js_class = Contract)]
impl WasmContract {
    /// Constructs a contract from a JS value (typically a plain object
    /// produced from a contract JSON document).
    ///
    /// The value is deserialized directly into `contrato::Contract` via
    /// the latter's `Deserialize` impl, which runs the full construction
    /// pipeline: children are loaded from the nested tree, `{{this.*}}`
    /// templates are interpolated, and the requirements index is built.
    #[wasm_bindgen(constructor)]
    pub fn new(value: JsValue) -> Result<WasmContract, JsValue> {
        let inner: Contract = serde_wasm_bindgen::from_value(value).map_err(wasm_err)?;
        Ok(WasmContract { inner })
    }

    // ── accessors ────────────────────────────────────────────────────────

    /// Returns the contract's type string (e.g. `"sw.os"`).
    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> String {
        self.inner.get_type().to_string()
    }

    /// Returns the contract's slug, or `undefined` if absent.
    #[wasm_bindgen(js_name = getSlug)]
    pub fn get_slug(&self) -> Option<String> {
        self.inner.get_slug().map(str::to_string)
    }

    /// Returns the contract's version (semver or identifier), or
    /// `undefined` if absent.
    #[wasm_bindgen(js_name = getVersion)]
    pub fn get_version(&self) -> Option<String> {
        self.inner.get_version()
    }

    /// Returns the contract's canonical slug, falling back to its own
    /// slug when no canonical slug is set.
    #[wasm_bindgen(js_name = getCanonicalSlug)]
    pub fn get_canonical_slug(&self) -> Option<String> {
        self.inner.get_canonical_slug().map(str::to_string)
    }

    /// Returns the contract's reference string: `"slug"` or
    /// `"slug@version"`.
    #[wasm_bindgen(js_name = getReferenceString)]
    pub fn get_reference_string(&self) -> String {
        self.inner.get_reference_string()
    }

    /// Returns every slug this contract can be referenced by (own slug
    /// plus every alias) as a JS array of strings.
    #[wasm_bindgen(js_name = getAllSlugs)]
    pub fn get_all_slugs(&self) -> Array {
        let arr = Array::new();
        for slug in self.inner.get_all_slugs() {
            arr.push(&JsValue::from_str(slug));
        }
        arr
    }

    /// Returns `true` if the contract has at least one alias.
    #[wasm_bindgen(js_name = hasAliases)]
    pub fn has_aliases(&self) -> bool {
        self.inner.has_aliases()
    }

    /// Returns the contract's deterministic hash.
    pub fn hash(&self) -> String {
        self.inner.hash().to_string()
    }

    /// Returns the full contract as a plain JS object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        self.inner.serialize(&json_serializer()).map_err(wasm_err)
    }

    /// Re-runs `{{this.*}}` template interpolation against the current
    /// state of the contract. Intended for JS callers that mutate the
    /// underlying data between constructions (for example by swapping
    /// in a new children set) and want the parent's template
    /// placeholders re-resolved against any field whose value changed.
    /// Invalidates the hash cache; the children subtree is left
    /// untouched — each child was already interpolated against its own
    /// fields at construction.
    pub fn interpolate(&mut self) {
        self.inner.interpolate();
    }

    // ── children management ─────────────────────────────────────────────

    /// Adds a single child contract.
    #[wasm_bindgen(js_name = addChild)]
    pub fn add_child(&mut self, child: &WasmContract) {
        self.inner.add_child(child.inner.clone());
    }

    /// Adds many child contracts in a single batch (move semantics).
    ///
    /// The JS handles passed in become unusable after the call.
    /// Efficient: no cloning, one rebuild.
    #[wasm_bindgen(js_name = addChildren)]
    pub fn add_children(&mut self, children: Vec<WasmContract>) {
        self.inner
            .add_children(children.into_iter().map(|c| c.inner));
    }

    /// Removes a child matching the given contract.
    #[wasm_bindgen(js_name = removeChild)]
    pub fn remove_child(&mut self, child: &WasmContract) {
        self.inner.remove_child(&child.inner);
    }

    /// Returns the direct child with the given hash, or `undefined`.
    #[wasm_bindgen(js_name = getChildByHash)]
    pub fn get_child_by_hash(&self, hash: &str) -> Option<WasmContract> {
        self.inner
            .get_child_by_hash(hash)
            .cloned()
            .map(|inner| WasmContract { inner })
    }

    /// Returns every reachable child (recursive).
    #[wasm_bindgen(js_name = getChildren)]
    pub fn get_children(&self) -> Array {
        contracts_to_array(self.inner.get_children().into_iter().cloned())
    }

    /// Returns every reachable child of the given type (recursive).
    #[wasm_bindgen(js_name = getChildrenByType)]
    pub fn get_children_by_type(&self, kind: &str) -> Array {
        contracts_to_array(self.inner.get_children_by_type(kind).into_iter().cloned())
    }

    /// Returns every reachable child whose type is in `kinds` (recursive).
    ///
    /// Filters on the Rust side using the type index, avoiding the
    /// serialization of children that would be discarded by a JS-side
    /// filter.
    #[wasm_bindgen(js_name = getChildrenByTypes)]
    pub fn get_children_by_types(&self, kinds: Vec<String>) -> Array {
        let refs: Vec<&str> = kinds.iter().map(String::as_str).collect();
        contracts_to_array(self.inner.get_children_filtered(&refs).into_iter().cloned())
    }

    /// Returns the deduplicated set of all child types reachable from
    /// this contract.
    #[wasm_bindgen(js_name = getChildrenTypes)]
    pub fn get_children_types(&self) -> Array {
        let arr = Array::new();
        for t in self.inner.get_children_types() {
            arr.push(&JsValue::from_str(&t));
        }
        arr
    }

    // ── search ──────────────────────────────────────────────────────────

    /// Searches for children matching the given matcher, populating the
    /// internal search cache on first lookup.
    ///
    /// `matcher` is a plain JS object shaped like `{ type, slug?,
    /// version?, data? }`, deserialized into [`ContractMatcher`] at the
    /// boundary.
    #[wasm_bindgen(js_name = findChildren)]
    pub fn find_children(&mut self, matcher: JsValue) -> Result<Array, JsValue> {
        let matcher = matcher_from_js(matcher)?;
        Ok(contracts_to_array(
            self.inner.find_children(&matcher).into_iter().cloned(),
        ))
    }

    // ── validation ──────────────────────────────────────────────────────

    /// Returns `true` if every compiled requirement of `contract` and
    /// its descendants is satisfied by children reachable from `self`.
    #[wasm_bindgen(js_name = satisfiesChildContract)]
    pub fn satisfies_child_contract(
        &mut self,
        contract: &WasmContract,
        types: Option<Vec<String>>,
    ) -> bool {
        let owned = borrow_type_filter(&types);
        self.inner
            .satisfies_child_contract(&contract.inner, owned.as_deref())
    }

    /// Returns the set of `contract`'s compiled requirements that are
    /// unsatisfied by children reachable from `self`, as an array of
    /// plain JS objects. Each entry has the same shape as a
    /// `requires` entry on the source contract.
    #[wasm_bindgen(js_name = getNotSatisfiedChildRequirements)]
    pub fn get_not_satisfied_child_requirements(
        &mut self,
        contract: &WasmContract,
        types: Option<Vec<String>>,
    ) -> Result<JsValue, JsValue> {
        let owned = borrow_type_filter(&types);
        let result = self
            .inner
            .get_not_satisfied_child_requirements(&contract.inner, owned.as_deref());
        result.serialize(&json_serializer()).map_err(wasm_err)
    }

    /// Returns `true` if every direct and descendant child of `self`
    /// has its compiled requirements satisfied against `self` itself.
    #[wasm_bindgen(js_name = areChildrenSatisfied)]
    pub fn are_children_satisfied(&mut self, types: Option<Vec<String>>) -> bool {
        let owned = borrow_type_filter(&types);
        self.inner.are_children_satisfied(owned.as_deref())
    }

    /// Aggregates every unsatisfied compiled requirement across all
    /// descendants of `self`, as an array of plain JS objects.
    #[wasm_bindgen(js_name = getAllNotSatisfiedChildRequirements)]
    pub fn get_all_not_satisfied_child_requirements(
        &mut self,
        types: Option<Vec<String>>,
    ) -> Result<JsValue, JsValue> {
        let owned = borrow_type_filter(&types);
        let result = self
            .inner
            .get_all_not_satisfied_child_requirements(owned.as_deref());
        result.serialize(&json_serializer()).map_err(wasm_err)
    }

    // ── requirements-index read accessors ────────────────────────────────

    /// Returns the distinct contract types this contract's own
    /// `requires` entries reference, as an array of strings.
    ///
    /// Only direct requirements are reported — descendants are not
    /// walked. Intended for JS callers that need to short-circuit a
    /// cross-reference walk when the contract has no opinion about a
    /// given set of types, before iterating the matcher buckets
    /// returned by [`Self::get_requirement_matchers_for_type`].
    #[wasm_bindgen(js_name = getRequirementTypes)]
    pub fn get_requirement_types(&self) -> Array {
        let arr = Array::new();
        for t in self.inner.requirement_types() {
            arr.push(&JsValue::from_str(t));
        }
        arr
    }

    /// Returns every simple matcher registered under the given
    /// requirement type, as an array of plain JS objects.
    ///
    /// Each object has the same shape as a matcher argument to
    /// [`Self::find_children`] and can be handed straight back into
    /// that method — the intended use is to walk cross-references by
    /// resolving each bucketed matcher against a parent contract. The
    /// returned array is empty when the type is absent from this
    /// contract's requirements index.
    #[wasm_bindgen(js_name = getRequirementMatchersForType)]
    pub fn get_requirement_matchers_for_type(&self, kind: &str) -> Result<Array, JsValue> {
        let serializer = json_serializer();
        let arr = Array::new();
        for m in self.inner.requirement_matchers_for_type(kind) {
            arr.push(&m.serialize(&serializer).map_err(wasm_err)?);
        }
        Ok(arr)
    }

    // ── statics ─────────────────────────────────────────────────────────

    /// Expands a source contract into one or more concrete contracts
    /// by running variant expansion and alias generation.
    ///
    /// Accepts a JS value deserialized into [`RawContract`]. Returns an
    /// array of fresh `Contract` handles.
    #[wasm_bindgen(js_name = build)]
    pub fn build(source: JsValue) -> Result<Array, JsValue> {
        let raw: RawContract = serde_wasm_bindgen::from_value(source).map_err(wasm_err)?;
        Ok(contracts_to_array(Contract::build(&raw)))
    }

    /// Returns `true` when two contracts have the same deterministic
    /// hash (i.e. represent structurally equivalent data).
    #[wasm_bindgen(js_name = isEqual)]
    pub fn is_equal(a: &WasmContract, b: &WasmContract) -> bool {
        a.inner == b.inner
    }
}
