//! Core [`Contract`] type, constructor, accessors, and static helpers.
//!
//! A [`Contract`] wraps a [`RawContract`] with derived state: a stable hash,
//! a children index (populated from the nested tree in `raw.children`), and
//! a requirements index (populated from `raw.requires`). Template
//! interpolation is applied at construction time so that `{{this.*}}`
//! expressions are resolved against the contract's own fields.
//!
//! Contracts are constructed by deserializing from JSON (see the
//! [`serde::Deserialize`] impl) or via [`Contract::build`], which expands
//! variants and aliases. Direct mutation of the underlying raw data is not
//! supported; callers round-trip through JSON if they need to transform a
//! contract. Children are managed through `add_child` / `remove_child` /
//! `add_children`, which keep the derived state (children tree,
//! requirements index, cached hash) in sync after every mutation.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::OnceLock;

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use serde_json::Value;

use crate::children_tree;
use crate::hash::hash_object;
use crate::index::ContractIndex;
use crate::matcher::{partial_match, version_match};
use crate::matcher_cache::MatcherCache;
use crate::object_set::{Identifiable, ObjectSet};
use crate::template;
use crate::types::{ContractMatcher, ContractRequirement, RawContract, Slug, VersionReq};
use crate::variants;

/// Compiled requirements derived from `raw.requires`.
///
/// Each entry in `raw.requires` is registered as strongly typed
/// [`ContractMatcher`] and [`ContractRequirement`] values — simple `Match`
/// entries contribute their inner matcher directly, while `Or` / `Not`
/// entries contribute every inner matcher plus the boolean requirement
/// itself. The two `ObjectSet`s deduplicate their entries via
/// [`Identifiable`], so duplicate `requires` entries collapse to a single
/// stored matcher / requirement.
#[derive(Debug, Clone, Default)]
struct RequirementsIndex {
    /// Per-type set of registered simple matchers.
    ///
    /// Populated from every `Match` entry in `raw.requires` and from every
    /// inner matcher of an `Or` / `Not` entry. Gives requirement-satisfaction
    /// checks a flat list of matchers per target type without re-walking
    /// the `compiled` set to unwrap boolean operations.
    matchers: HashMap<String, ObjectSet<ContractMatcher>>,

    /// Set of contract types referenced by any registered matcher.
    types: HashSet<String>,

    /// Flat set of top-level compiled requirements.
    ///
    /// For simple `Match` entries this holds one requirement per `requires`
    /// entry. For `Or` / `Not` entries it holds one boolean-operation
    /// requirement whose inner matchers are also indexed in `matchers`.
    compiled: ObjectSet<ContractRequirement>,
}

/// A contract: raw data plus derived hash, children index, and requirements.
///
/// The preferred way to construct a [`Contract`] is via [`serde`]
/// deserialization from JSON, or via [`Contract::build`] for variant + alias
/// expansion. Construction interpolates `{{this.*}}` templates and
/// rebuilds the children tree and requirements index; the hash itself is
/// computed lazily on first call to [`Self::hash`].
#[derive(Clone)]
pub struct Contract {
    /// The raw contract data.
    raw: RawContract,

    /// Lazily computed deterministic hash of `raw`.
    ///
    /// Populated on first call to [`Self::hash`] (or any path that
    /// reaches it — `PartialEq`, `std::hash::Hash`, `Identifiable::id`,
    /// [`ChildrenIndex::insert`](crate::children::ChildrenIndex)). Any
    /// mutation that changes `raw` must invalidate this cell; in
    /// practice [`Self::rebuild`] does so for every mutator, so callers
    /// only need to call `rebuild` after touching `raw`.
    hash: OnceLock<String>,

    /// Indexed children contracts, populated from `raw.children` at
    /// construction time and kept in sync via [`Self::rebuild`].
    children: ContractIndex,

    /// Matcher-based search result cache keyed by `(target type,
    /// matcher hash)`.
    ///
    /// A sibling field of [`Self::children`] so the search path can
    /// split-borrow `&self.children` (read-only index walk) and
    /// `&mut self.search_cache` (mutable cache write) in the same
    /// expression without the borrow checker forcing an
    /// intermediate clone. [`Self::find_children`] exploits the
    /// split to resolve cached hashes into `&Contract` references
    /// and then move the hash set into the cache in one pass; the
    /// requirement-satisfaction loops exploit it to iterate every
    /// descendant's compiled requirements in place without an
    /// owned per-descendant snapshot.
    ///
    /// Invalidation is driven by [`Self::add_child`] /
    /// [`Self::remove_child`] / [`Self::add_children`]: those
    /// mutators read the affected types out of the
    /// [`crate::children::Mutation`] result returned by
    /// [`ChildrenIndex`] and drop the matching cache entries via
    /// [`Self::invalidate_search_cache_for`].
    search_cache: MatcherCache<HashSet<String>>,

    /// Compiled requirements derived from `raw.requires`.
    requirements: RequirementsIndex,
}

/// Construction and core lifecycle.
impl Contract {
    /// Constructs a [`Contract`] from a [`RawContract`].
    ///
    /// Children are loaded from `raw.children`, templates are
    /// interpolated, and the requirements index is rebuilt. The
    /// contract's own hash is **not** computed at this point — it is
    /// populated lazily on first call to [`Self::hash`]. Blueprint's
    /// combinatorial expansion relies on this: ephemeral parent
    /// contracts that are only used for satisfiability checks never pay
    /// the hashing cost.
    pub(crate) fn new(raw: RawContract) -> Self {
        let mut this = Self {
            raw,
            hash: OnceLock::new(),
            children: ContractIndex::default(),
            search_cache: MatcherCache::new(),
            requirements: RequirementsIndex::default(),
        };

        // Load children from the nested tree and insert each one into
        // the index. Children are hashed on insertion because the index
        // keys by hash; that cost is unavoidable for any indexed child.
        // The cache starts empty, so the `Mutation::invalidated_types`
        // list returned by `insert` is discarded here — there's
        // nothing to invalidate at construction.
        let child_sources: Vec<RawContract> = this
            .raw
            .body
            .children
            .as_ref()
            .map(children_tree::get_all)
            .unwrap_or_default();

        for source in child_sources {
            // `Mutation::invalidated_types` is deliberately discarded:
            // the search cache starts empty at construction, so there
            // is nothing to invalidate. `let _ =` silences the
            // `#[must_use]` warning while making the intent explicit.
            let _ = this.children.insert(Contract::new(source));
        }

        // Convert `provides` capabilities into child contracts, removing the property from the raw
        // body. This is kept for backwards compatibility with earlier versions of the contracts specification
        for capability in std::mem::take(&mut this.raw.body.provides) {
            let child_raw = RawContract {
                kind: capability.kind,
                body: capability.body,
                ..RawContract::default()
            };
            let _ = this.children.insert(Contract::new(child_raw));
        }

        this.interpolate();
        this
    }

    /// Compiles `{{this.*}}` templates in `raw`, then rebuilds derived
    /// state.
    ///
    /// Runs automatically during construction. Exposed publicly so that
    /// callers which mutate the contract afterwards — typically by
    /// replacing the children index with a new set and wanting the
    /// parent's `{{this.*}}` placeholders re-evaluated against any
    /// field that changed — can force a fresh pass. Repeated calls are
    /// safe: unresolved placeholders stay unresolved and
    /// already-resolved values stay stable as long as `raw` is
    /// unchanged.
    ///
    /// The `children` subtree is excluded from interpolation: each
    /// child is its own contract and is interpolated against its own
    /// fields during its own construction. [`Self::rebuild`] is called
    /// at the end, which invalidates the hash cell.
    pub fn interpolate(&mut self) {
        let mut blacklist = HashSet::new();
        blacklist.insert("children".to_string());

        let raw_value =
            serde_json::to_value(&self.raw).expect("RawContract must serialize to JSON");
        let compiled = template::compile_contract(&raw_value, &blacklist, None);
        self.raw = serde_json::from_value(compiled)
            .expect("compiled contract must deserialize into RawContract");

        self.rebuild();
    }

    /// Rebuilds derived state from the current children index and
    /// `raw.requires`.
    ///
    /// - Regenerates `raw.children` from the children index. When the
    ///   index is empty, `raw.children` is cleared to `None` so that the
    ///   serialized form stays in sync with the index — important when
    ///   [`Self::remove_child`] empties the index.
    /// - Resets and repopulates the requirements index.
    /// - Invalidates the lazy hash cell, since `raw` has potentially
    ///   changed.
    fn rebuild(&mut self) {
        self.raw.body.children = if self.children.is_empty() {
            None
        } else {
            Some(
                children_tree::build(&self.children)
                    .expect("children tree must build without path conflicts"),
            )
        };

        let mut requirements = RequirementsIndex::default();
        for conjunct in &self.raw.body.requires {
            Self::register_requirement(&mut requirements, conjunct);
        }
        self.requirements = requirements;

        // Drop any previously computed hash so the next `hash()` call
        // recomputes against the updated `raw`. `take()` returns the
        // old value without reconstructing the struct field; the
        // discarded `String` is freed in-place.
        let _ = self.hash.take();
    }

    /// Registers a single top-level requirement into the requirements index.
    ///
    /// For a `Match` entry the inner [`ContractMatcher`] is inserted into
    /// `matchers[kind]` (deduplicated by [`Identifiable`]) and the whole
    /// requirement is inserted into `compiled`. For an `Or` / `Not` entry
    /// every inner matcher is inserted into `matchers[kind]` so the
    /// satisfaction check can iterate per-type matchers without re-walking
    /// the operation node, and the original `ContractRequirement` is
    /// inserted into `compiled` so boolean semantics are preserved at
    /// validation time.
    ///
    /// Nested boolean operations cannot reach this function: the inner
    /// type of `Or` / `Not` is [`ContractMatcher`], not
    /// [`ContractRequirement`], so the type system rules out
    /// `{or: [{or: [...]}]}` shapes at deserialization.
    fn register_requirement(index: &mut RequirementsIndex, req: &ContractRequirement) {
        match req {
            ContractRequirement::Match(m) => {
                Self::register_matcher(index, m);
            }
            ContractRequirement::Or(items) | ContractRequirement::Not(items) => {
                for m in items {
                    Self::register_matcher(index, m);
                }
            }
        }
        index.compiled.insert(req.clone());
    }

    /// Inserts a single [`ContractMatcher`] into `matchers[kind]` and
    /// updates the known-types set.
    ///
    /// Shared by the `Match` and `Or` / `Not` arms of
    /// [`Self::register_requirement`] so both code paths agree on
    /// deduplication semantics and on the known-types invariant.
    ///
    /// Allocates the type string exactly once: the clone goes into
    /// the known-types set and the original is consumed by the
    /// `matchers` `HashMap::entry` call. Inserting into `matchers`
    /// first and then cloning for `types` would cost two clones
    /// because `entry` takes ownership of the key.
    fn register_matcher(index: &mut RequirementsIndex, matcher: &ContractMatcher) {
        let ty = matcher.kind.as_str().to_string();
        index.types.insert(ty.clone());
        index
            .matchers
            .entry(ty)
            .or_default()
            .insert(matcher.clone());
    }
}

/// Accessors.
impl Contract {
    /// Returns the contract's type string (e.g., `"sw.os"`).
    pub fn get_type(&self) -> &str {
        self.raw.kind.as_str()
    }

    /// Returns the contract's slug, or `None` if absent.
    pub fn get_slug(&self) -> Option<&str> {
        self.raw.body.slug.as_ref().map(Slug::as_str)
    }

    /// Returns the contract's version as a string, or `None` if absent.
    ///
    /// Semver versions are returned in their normalized form; identifier
    /// versions (e.g. `"wheezy"`) are returned as-is.
    pub fn get_version(&self) -> Option<String> {
        self.raw.body.version.as_ref().map(ToString::to_string)
    }

    /// Returns the canonical slug, falling back to the contract's slug.
    ///
    /// Alias contracts generated by [`Contract::build`] set `canonical_slug`
    /// to the original slug; for non-alias contracts this returns the same
    /// value as [`Self::get_slug`].
    pub fn get_canonical_slug(&self) -> Option<&str> {
        self.raw
            .canonical_slug
            .as_ref()
            .map(Slug::as_str)
            .or_else(|| self.get_slug())
    }

    /// Returns the contract's reference string in the form `slug` or
    /// `slug@version` depending on whether a version is present.
    pub fn get_reference_string(&self) -> String {
        let slug = self.get_slug().unwrap_or("");
        match self.get_version() {
            Some(v) => format!("{slug}@{v}"),
            None => slug.to_string(),
        }
    }

    /// Returns an iterator over all slugs this contract can be referenced
    /// by: its own slug (if any) together with every alias.
    ///
    /// Prefers borrowed `&str` over allocated `String` so callers that
    /// build indexes (e.g. [`crate::children::ChildrenIndex::insert`])
    /// can avoid one allocation per insertion.
    pub fn get_all_slugs(&self) -> impl Iterator<Item = &str> {
        self.raw
            .body
            .aliases
            .iter()
            .map(Slug::as_str)
            .chain(self.get_slug())
    }

    /// Returns `true` if the contract has at least one alias.
    pub fn has_aliases(&self) -> bool {
        !self.raw.body.aliases.is_empty()
    }

    /// Returns the contract's deterministic hash, computing it on
    /// first call and caching the result.
    ///
    /// The hash is a deterministic SHA-256 digest of the serialized raw
    /// contract data. Subsequent calls return the cached value without
    /// re-hashing. Any mutation that routes through [`Self::rebuild`]
    /// (i.e. every mutator on [`Contract`]) invalidates the cache, so
    /// the next call recomputes.
    pub fn hash(&self) -> &str {
        self.hash.get_or_init(|| {
            let value =
                serde_json::to_value(&self.raw).expect("RawContract must serialize to JSON");
            hash_object(&value)
        })
    }

    /// Returns an iterator over the distinct contract types referenced
    /// by this contract's own `requires` entries.
    ///
    /// Only this contract's direct requirements are reported; the walk
    /// does **not** descend into children. Intended for callers that
    /// need to decide whether a given contract has any opinion about a
    /// set of types — typically to short-circuit a cross-reference
    /// resolution or a filter pass before iterating the matcher
    /// buckets returned by [`Self::requirement_matchers_for_type`].
    pub fn requirement_types(&self) -> impl Iterator<Item = &str> {
        self.requirements.types.iter().map(String::as_str)
    }

    /// Returns an iterator over the simple matchers registered under
    /// the given requirement type.
    ///
    /// Returns an empty iterator when the type is not present in this
    /// contract's own requirements index. The matchers yielded are the
    /// compiled form of this contract's `requires` entries, bucketed
    /// by target type; feeding each one back into
    /// [`Self::find_children`] on a parent contract walks the
    /// cross-references a requirement entry induces.
    pub fn requirement_matchers_for_type(
        &self,
        kind: &str,
    ) -> impl Iterator<Item = &ContractMatcher> {
        self.requirements
            .matchers
            .get(kind)
            .into_iter()
            .flat_map(|set| set.values())
    }

    /// Returns a reference to the underlying [`RawContract`].
    ///
    /// Crate-internal accessor used by [`children_tree::build`] via the
    /// [`crate::children_tree::WithChildrenIndex`] trait implementation
    /// on [`crate::children::ChildrenIndex`].
    pub(crate) fn raw(&self) -> &RawContract {
        &self.raw
    }
}

/// Static helpers.
impl Contract {
    /// Expands a source contract into one or more concrete contracts by
    /// applying variant expansion and alias generation.
    ///
    /// For each expanded variant:
    /// 1. One contract is produced for each alias, with `canonical_slug`
    ///    set to the original slug and `slug` replaced by the alias.
    /// 2. One contract is produced for the variant itself, with its
    ///    `aliases` cleared.
    ///
    /// The alias contracts come before the base contract in the output.
    pub fn build(source: &RawContract) -> Vec<Contract> {
        let mut result = Vec::new();
        for variant in variants::build(source) {
            let aliases: Vec<Slug> = variant.body.aliases.clone();
            let mut base = variant;
            base.body.aliases.clear();

            for alias in &aliases {
                let mut alias_contract = base.clone();
                alias_contract.canonical_slug = base.body.slug.clone();
                alias_contract.body.slug = Some(alias.clone());
                result.push(Contract::new(alias_contract));
            }
            result.push(Contract::new(base));
        }
        result
    }
}

/// Children management.
impl Contract {
    /// Adds a child contract to this contract.
    ///
    /// When the child is new, the derived state (`raw.children`
    /// serialized tree and the requirements index) is rebuilt, every
    /// search-cache entry for the affected types is dropped, and the
    /// parent's hash cache is invalidated so the next [`Self::hash`]
    /// call recomputes. Adding a child whose hash is already present in
    /// the index is a full no-op — the indexes, serialized tree,
    /// search cache, and cached hash are all unchanged — which keeps
    /// duplicate-insert traffic cheap in blueprint's combinatorial
    /// hot loop.
    pub fn add_child(&mut self, contract: Contract) -> &mut Self {
        let mutation = self.children.insert(contract);
        if mutation.changed {
            self.invalidate_search_cache_for(&mutation.invalidated_types);
            self.rebuild();
        }
        self
    }

    /// Removes a child contract from this contract.
    ///
    /// The child is identified by its hash: any [`Contract`] hashing to
    /// the same value as a stored child will remove that child. When the
    /// child is not present the call is a no-op and the derived state is
    /// left untouched. Otherwise the children tree and requirements
    /// index are rebuilt (which also invalidates the parent's hash
    /// cache) and every search-cache entry for the affected types is
    /// dropped.
    pub fn remove_child(&mut self, contract: &Contract) -> &mut Self {
        let mutation = self.children.remove(contract);
        if mutation.changed {
            self.invalidate_search_cache_for(&mutation.invalidated_types);
            self.rebuild();
        }
        self
    }

    /// Adds multiple child contracts to this contract.
    ///
    /// All insertions happen first (accumulating the union of
    /// invalidated types across the batch), then a single
    /// [`Self::rebuild`] is performed — avoiding the O(n²) cost of
    /// rebuilding after each individual insertion. Duplicate children
    /// (by hash) within the batch are deduplicated through the same
    /// no-op path used by single insertion. If every contract in the
    /// batch turns out to be a duplicate (or the batch is empty), no
    /// `rebuild` is performed, no search-cache entries are dropped,
    /// and the parent's hash cache is left intact.
    pub fn add_children(&mut self, contracts: impl IntoIterator<Item = Contract>) -> &mut Self {
        let mut invalidated: Vec<String> = Vec::new();
        for contract in contracts {
            let mutation = self.children.insert(contract);
            if mutation.changed {
                invalidated.extend(mutation.invalidated_types);
            }
        }
        if !invalidated.is_empty() {
            self.invalidate_search_cache_for(&invalidated);
            self.rebuild();
        }
        self
    }

    /// Drops every search-cache entry keyed on any type in `types`.
    ///
    /// Called by the three mutators ([`Self::add_child`],
    /// [`Self::remove_child`], [`Self::add_children`]) whenever the
    /// children index actually changed. The input may contain
    /// duplicates — `add_children` in particular concatenates
    /// per-insertion invalidation lists without upfront
    /// deduplication. A small `HashSet<&str>` probe filters
    /// duplicates before touching [`MatcherCache::remove`] so a
    /// batch of N children of the same type performs exactly one
    /// cache removal per distinct type instead of N. The probe
    /// itself costs one [`HashSet`] insertion per entry, which is
    /// cheaper than a redundant [`HashMap::remove`] on the cache's
    /// own internal map.
    fn invalidate_search_cache_for(&mut self, types: &[String]) {
        let mut seen: HashSet<&str> = HashSet::new();
        for ty in types {
            if seen.insert(ty.as_str()) {
                self.search_cache.remove(ty);
            }
        }
    }

    /// Looks up a direct child contract by its hash.
    ///
    /// This is a non-recursive lookup — only direct children of `self`
    /// are considered, not children of children.
    pub fn get_child_by_hash(&self, child_hash: &str) -> Option<&Contract> {
        self.children.get(child_hash)
    }

    /// Recursively collects all children of this contract.
    ///
    /// Returns direct children of `self` and, for each of those,
    /// recurses into their own children. A contract with no children
    /// returns an empty vector. Use [`Self::get_children_filtered`] to
    /// constrain the result to a fixed set of types.
    pub fn get_children(&self) -> Vec<&Contract> {
        let mut out = Vec::new();
        self.collect_children(&[], &mut out);
        out
    }

    /// Recursively collects children whose type is in `types`.
    ///
    /// Non-matching children are still traversed so that their own
    /// matching descendants are returned — filtering prunes the
    /// emitted set, not the walk.
    ///
    /// The filter is a slice rather than a set: typical call sites pass
    /// one or a handful of types, so linear scan is both faster than a
    /// `HashSet` lookup and zero-allocation at the call site. Duplicate
    /// entries in `types` are harmless — each child is still emitted at
    /// most once per visit.
    pub fn get_children_filtered(&self, types: &[&str]) -> Vec<&Contract> {
        let mut out = Vec::new();
        self.collect_children(types, &mut out);
        out
    }

    /// Shared recursion for [`Self::get_children`] and
    /// [`Self::get_children_filtered`].
    ///
    /// An empty `types` slice means "no filter" and every visited
    /// child is pushed. Otherwise, only children whose type appears in
    /// the slice are pushed, but recursion always continues into every
    /// child so that matching descendants under a non-matching parent
    /// are still surfaced.
    fn collect_children<'a>(&'a self, types: &[&str], out: &mut Vec<&'a Contract>) {
        for child in self.children.values() {
            if types.is_empty() || types.contains(&child.get_type()) {
                out.push(child);
            }
            child.collect_children(types, out);
        }
    }

    /// Recursively collects children whose type matches `type_`.
    ///
    /// Convenience shorthand for [`Self::get_children_filtered`] with a
    /// single-element slice. Callers who need cached, matcher-keyed
    /// lookup should use [`Self::find_children`] instead.
    pub fn get_children_by_type(&self, type_: &str) -> Vec<&Contract> {
        self.get_children_filtered(&[type_])
    }

    /// Returns the deduplicated list of child contract types reachable
    /// from this contract, including every descendant's direct types.
    ///
    /// A single `HashSet` accumulator is threaded through the recursion
    /// to deduplicate in place; the final set is converted to a `Vec`
    /// at the top of the call. The output order is unspecified.
    pub fn get_children_types(&self) -> Vec<String> {
        let mut acc = HashSet::new();
        self.collect_children_types_into(&mut acc);
        acc.into_iter().collect()
    }

    /// Recursive helper for [`Self::get_children_types`] that extends
    /// the given accumulator with this contract's direct child types
    /// and every descendant's types. Avoids the per-node allocations
    /// of the naive "return a fresh `HashSet` and `extend`" pattern.
    fn collect_children_types_into(&self, acc: &mut HashSet<String>) {
        for ty in self.children.types() {
            acc.insert(ty.to_string());
        }
        for child in self.children.values() {
            child.collect_children_types_into(acc);
        }
    }
}

/// Matcher-based child search.
impl Contract {
    /// Recursively finds children matching the given matcher.
    ///
    /// The matcher is a typed [`ContractMatcher`] with a required
    /// target type plus optional slug, semver requirement, and a
    /// `data` payload for deep partial matching.
    ///
    /// The search walks `self` together with every descendant. For
    /// each visited contract, the candidate set is drawn from that
    /// contract's own children index, narrowed first by `(type,
    /// slug)` when the matcher specifies a slug and by `type` alone
    /// otherwise. Each candidate is then filtered by [`partial_match`]
    /// over the matcher's `data` (against the child's own `data`) and
    /// by [`version_match`] over the matcher's version requirement.
    ///
    /// Results are cached on `self` keyed by `(target_type,
    /// matcher_hash)`, where `matcher_hash` is the
    /// [`Identifiable`] digest of the typed matcher — the same digest
    /// the requirements index uses for deduplication. Subsequent
    /// calls with the same matcher return the same contracts (modulo
    /// mutation). The cache is invalidated per-type whenever a child
    /// of that type is added or removed from any node whose index
    /// participates in the walk.
    ///
    /// Returns an empty vector when the target type is not present
    /// in any descendant of `self`.
    ///
    /// This method takes `&mut self` because populating the search
    /// cache is a mutation. Callers that want to hold the result
    /// list past further operations should convert the borrowed
    /// slice to owned hashes or clone the contracts.
    pub fn find_children(&mut self, matcher: &ContractMatcher) -> Vec<&Contract> {
        let target_type = matcher.kind.as_str();

        // Short-circuit before touching the cache: if no contract
        // in the walk can possibly contribute a match, the answer
        // is empty regardless of cache state.
        if !Self::has_descendant_type_in(&self.children, target_type) {
            return Vec::new();
        }

        // Cache hit: the cached `HashSet<String>` is borrowed from
        // `self.search_cache`, and `resolve_hashes_in` walks
        // `self.children` — two sibling fields on `self`, so the
        // borrow checker allows them to coexist via field-level
        // split. The cached set is resolved into `&Contract`
        // references directly, no intermediate clone.
        if let Some(cached) = self.search_cache.get(matcher) {
            return Self::resolve_hashes_in(&self.children, cached);
        }

        let target_slug = matcher.slug.as_ref().map(Slug::as_str);

        let mut result_hashes = HashSet::new();
        Self::collect_find_children_hashes_walk(
            &self.children,
            target_type,
            target_slug,
            matcher.version.as_ref(),
            matcher,
            &mut result_hashes,
        );

        // Resolve against the read borrow of `self.children` first,
        // then move `result_hashes` into the cache. The mutable
        // borrow of `self.search_cache` is disjoint from the
        // immutable borrow of `self.children`, so the owned result
        // and the cache insert coexist with no clone.
        let resolved = Self::resolve_hashes_in(&self.children, &result_hashes);
        self.search_cache.insert(matcher, result_hashes);
        resolved
    }

    /// Returns `true` when `ty` appears anywhere in the closure of
    /// child types reachable from `children`.
    ///
    /// Used as the first short-circuit inside [`Self::find_children`]
    /// and [`Self::any_child_matches_in`]. The recursive probe stops
    /// on the first positive match, so the check costs only the
    /// prefix of the walk needed to discover one child of the
    /// requested type.
    ///
    /// Takes `&ChildrenIndex` directly rather than `&self` so the
    /// caller can pass `&self.children` while simultaneously holding
    /// `&mut self.search_cache` — field-level split borrow is the
    /// pivot that lets the validation descendants walk iterate
    /// compiled requirements in place, without an owned
    /// per-descendant snapshot.
    fn has_descendant_type_in(children: &ContractIndex, ty: &str) -> bool {
        if children.has_type(ty) {
            return true;
        }
        for child in children.values() {
            if Self::has_descendant_type_in(&child.children, ty) {
                return true;
            }
        }
        false
    }

    /// Collects child hashes matched by [`Self::find_children`]
    /// across the given index and every descendant.
    ///
    /// Visits the passed-in [`ChildrenIndex`] as a candidate parent
    /// (inspecting its direct children via the type / type+slug
    /// indexes), then recurses into each direct child's own
    /// children index so descendants also contribute candidates.
    /// Recursion is effectively unrolled into "visit every node as
    /// its own candidate", matching the semantics of the original
    /// `get_children()`-based sweep while avoiding the intermediate
    /// `Vec<&Contract>` allocation.
    fn collect_find_children_hashes_walk(
        children: &ContractIndex,
        target_type: &str,
        target_slug: Option<&str>,
        required_version: Option<&VersionReq>,
        matcher: &ContractMatcher,
        out: &mut HashSet<String>,
    ) {
        if children.has_type(target_type) {
            // The two candidate iterators have distinct concrete
            // types (one comes from `by_type_slug`, the other from
            // `by_type`), so the branch duplicates the surrounding
            // loop rather than unifying them behind a `dyn` box.
            // The inner per-candidate work lives in
            // [`Self::try_collect_candidate`] so neither branch
            // carries the full filter body.
            match target_slug {
                Some(slug) => {
                    for child_hash in children.hashes_by_type_slug(target_type, slug) {
                        Self::try_collect_candidate(
                            children,
                            child_hash,
                            required_version,
                            matcher.data.as_ref(),
                            out,
                        );
                    }
                }
                None => {
                    for child_hash in children.hashes_by_type(target_type) {
                        Self::try_collect_candidate(
                            children,
                            child_hash,
                            required_version,
                            matcher.data.as_ref(),
                            out,
                        );
                    }
                }
            }
        }
        for child in children.values() {
            Self::collect_find_children_hashes_walk(
                &child.children,
                target_type,
                target_slug,
                required_version,
                matcher,
                out,
            );
        }
    }

    /// Inserts `child_hash` into `out` when the child at that hash
    /// passes the `data` partial-match and version filters.
    ///
    /// Shared between the `(type, slug)` and `type`-only arms of
    /// [`Self::collect_find_children_hashes_walk`]. When the matcher
    /// carries no `data` payload, the partial-match check is
    /// skipped — type and slug have already been handled by the
    /// index lookup, so a data-less matcher accepts every candidate
    /// that reached this point.
    ///
    /// **Partial-match scope**: the matcher's `data` payload is
    /// matched against the child's own `data` field only. No other
    /// fields on the child contract are consulted during partial
    /// matching — custom criteria on either side belong inside
    /// `data`. A child with no `data` field cannot satisfy a
    /// non-trivial pattern and is rejected.
    fn try_collect_candidate(
        children: &ContractIndex,
        child_hash: &str,
        required_version: Option<&VersionReq>,
        pattern: Option<&Value>,
        out: &mut HashSet<String>,
    ) {
        let Some(child) = children.get(child_hash) else {
            return;
        };

        if let Some(pat) = pattern {
            // A child with no `data` of its own cannot satisfy a
            // non-trivial pattern — reject without running
            // `partial_match`.
            let Some(child_data) = child.raw.body.data.as_ref() else {
                return;
            };
            if !partial_match(pat, child_data) {
                return;
            }
        }

        if !version_match(child.raw.body.version.as_ref(), required_version) {
            return;
        }

        out.insert(child.hash().to_string());
    }

    /// Resolves a set of child hashes into `&Contract` references by
    /// walking a [`ChildrenIndex`] directly.
    ///
    /// Taking `&ChildrenIndex` (rather than `&self`) is the
    /// split-borrow pivot: the returned references are tied to the
    /// children index only, leaving `self.search_cache` free to be
    /// borrowed mutably in the same expression. That's what lets
    /// [`Self::find_children`] hand the owned `result_hashes` to
    /// `search_cache.insert` immediately after resolving, with no
    /// intermediate clone.
    ///
    /// The walk recurses into every descendant. `self` itself is
    /// **not** reachable from this function, so unlike the old
    /// `&self`-taking variant the root contract's own hash is never
    /// matched — that's fine because no current caller places the
    /// root's hash in the result set.
    fn resolve_hashes_in<'a>(
        children: &'a ContractIndex,
        hashes: &HashSet<String>,
    ) -> Vec<&'a Contract> {
        if hashes.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<&Contract> = Vec::with_capacity(hashes.len());
        Self::resolve_hashes_walk(children, hashes, &mut out);
        out
    }

    /// Recursive helper for [`Self::resolve_hashes_in`]: visits every
    /// contract in the index and its descendants, pushing any whose
    /// hash appears in `hashes`.
    fn resolve_hashes_walk<'a>(
        children: &'a ContractIndex,
        hashes: &HashSet<String>,
        out: &mut Vec<&'a Contract>,
    ) {
        for child in children.values() {
            if hashes.contains(child.hash()) {
                out.push(child);
            }
            Self::resolve_hashes_walk(&child.children, hashes, out);
        }
    }

    /// Boolean short-circuit variant of [`Self::find_children`].
    ///
    /// Returns `true` as soon as any child across the full descendant
    /// walk satisfies the matcher. Cache interaction is identical to
    /// [`Self::find_children`]: a cache hit returns the cached set's
    /// emptiness directly, and a cache miss computes and stores the
    /// full hash set (so subsequent
    /// [`find_children`](Self::find_children) /
    /// [`any_child_matches_in`](Self::any_child_matches_in) calls
    /// with the same matcher are served from the cache). The only
    /// saving versus `find_children` is the final `resolve_hashes_in`
    /// step — no `Vec<&Contract>` is ever materialized on the hot
    /// validation path.
    ///
    /// Takes `&ChildrenIndex` + `&mut MatcherCache` rather than
    /// `&mut self` so the caller can split-borrow sibling fields on
    /// [`Contract`] and keep other immutable borrows of `self` live
    /// across the call.
    fn any_child_matches_in(
        children: &ContractIndex,
        cache: &mut MatcherCache<HashSet<String>>,
        matcher: &ContractMatcher,
    ) -> bool {
        let target_type = matcher.kind.as_str();

        if !Self::has_descendant_type_in(children, target_type) {
            return false;
        }

        if let Some(hashes) = cache.get(matcher) {
            return !hashes.is_empty();
        }

        let target_slug = matcher.slug.as_ref().map(Slug::as_str);

        let mut result_hashes = HashSet::new();
        Self::collect_find_children_hashes_walk(
            children,
            target_type,
            target_slug,
            matcher.version.as_ref(),
            matcher,
            &mut result_hashes,
        );

        let non_empty = !result_hashes.is_empty();
        cache.insert(matcher, result_hashes);
        non_empty
    }
}

/// Requirement satisfaction.
///
/// The four public entry points (`satisfies_child_contract`,
/// `get_not_satisfied_child_requirements`, `are_children_satisfied`,
/// `get_all_not_satisfied_child_requirements`) all take `&mut self`
/// so they can drive the search-cache mutation inside
/// [`Self::any_child_matches_in`]. Internally each entry point
/// opens a **field-level split borrow** of two sibling
/// [`Contract`] fields — `&self.children` (read-only index walk)
/// and `&mut self.search_cache` (mutable cache write) — and threads
/// those references through a set of associated helpers
/// ([`Self::is_requirement_satisfied_in`],
/// [`Self::any_child_matches_in`]).
///
/// That split is what lets the descendant walks iterate
/// `descendant.requirements.compiled.values()` **directly**, with
/// no owned per-descendant snapshot in front of the satisfaction
/// loop. A snapshot-based design would clone every descendant's
/// compiled requirement list up front
/// (O(descendants × compiled_per_child) `ContractRequirement`
/// clones per call); the split-borrow version clones nothing on
/// the happy path.
impl Contract {
    /// Checks whether `contract`'s compiled requirements are all
    /// satisfied by the children (and capabilities) reachable from
    /// `self`.
    ///
    /// The conjuncts checked are `contract`'s own compiled
    /// requirements plus the compiled requirements of every
    /// descendant of **`contract`** (not of `self`). Iteration
    /// short-circuits on the first unsatisfied conjunct. An empty
    /// conjunct list is always satisfied.
    ///
    /// The `types` filter, when supplied, restricts which requirement
    /// types are evaluated: a simple `Match` whose type is not in the
    /// filter is treated as satisfied; `Or` / `Not` disjuncts are first
    /// filtered by allowed types before the boolean logic runs. Pass
    /// `None` to evaluate every requirement. The slice is consumed
    /// by linear `contains` on every requirement-type check —
    /// duplicates are harmless, so callers need not dedupe.
    pub fn satisfies_child_contract(
        &mut self,
        contract: &Contract,
        types: Option<&[&str]>,
    ) -> bool {
        let children = &self.children;
        let cache = &mut self.search_cache;

        Self::check_contract_satisfied_recursive(children, cache, contract, types)
    }

    /// Recursive worker for [`Self::satisfies_child_contract`].
    ///
    /// Walks `contract`'s own compiled requirements, then recurses
    /// into every direct child so each descendant's compiled
    /// requirements are checked under the same satisfaction scope.
    /// Short-circuits on the first unsatisfied conjunct.
    fn check_contract_satisfied_recursive(
        children: &ContractIndex,
        cache: &mut MatcherCache<HashSet<String>>,
        contract: &Contract,
        types: Option<&[&str]>,
    ) -> bool {
        for req in contract.requirements.compiled.values() {
            if !Self::is_requirement_satisfied_in(children, cache, req, types) {
                return false;
            }
        }
        for child in contract.children.values() {
            if !Self::check_contract_satisfied_recursive(children, cache, child, types) {
                return false;
            }
        }
        true
    }

    /// Returns the list of `contract`'s compiled requirements that
    /// are not satisfied by the children (and capabilities) reachable
    /// from `self`.
    ///
    /// The conjunct set is identical to the one used by
    /// [`Self::satisfies_child_contract`]: `contract`'s own compiled
    /// requirements plus the compiled requirements of every
    /// descendant of **`contract`**. Unlike the satisfaction check,
    /// this method does not short-circuit — every conjunct is
    /// evaluated so that the full unsatisfied list can be reported.
    /// Returns an empty vector when the conjunct set is empty.
    ///
    /// The returned requirements are cloned from the compiled
    /// index; callers receive owned data and can freely drop the
    /// source contract without affecting the result. The `types`
    /// filter has the same semantics as on
    /// [`Self::satisfies_child_contract`].
    pub fn get_not_satisfied_child_requirements(
        &mut self,
        contract: &Contract,
        types: Option<&[&str]>,
    ) -> Vec<ContractRequirement> {
        let children = &self.children;
        let cache = &mut self.search_cache;

        let mut out = Vec::new();
        Self::collect_not_satisfied_recursive(children, cache, contract, types, &mut out);
        out
    }

    /// Recursive worker for
    /// [`Self::get_not_satisfied_child_requirements`].
    ///
    /// Walks `contract`'s compiled requirements together with every
    /// descendant's, cloning the unsatisfied subset into `out`.
    /// Does **not** short-circuit — all conjuncts are evaluated so
    /// the full unsatisfied list can be reported.
    fn collect_not_satisfied_recursive(
        children: &ContractIndex,
        cache: &mut MatcherCache<HashSet<String>>,
        contract: &Contract,
        types: Option<&[&str]>,
        out: &mut Vec<ContractRequirement>,
    ) {
        for req in contract.requirements.compiled.values() {
            if !Self::is_requirement_satisfied_in(children, cache, req, types) {
                out.push(req.clone());
            }
        }
        for child in contract.children.values() {
            Self::collect_not_satisfied_recursive(children, cache, child, types, out);
        }
    }

    /// Checks whether every descendant of `self` has its compiled
    /// requirements satisfied against `self`.
    ///
    /// Walks every descendant recursively. When a `types` filter is
    /// supplied, a descendant whose own requirement-types set is
    /// disjoint with `types` is skipped: **its own** compiled
    /// requirements are not checked, but the walk still recurses
    /// into its children so any satisfied-relevant grandchildren
    /// are evaluated. The disjoint check targets the descendant's
    /// direct requirements only — not the types referenced by its
    /// subtree.
    ///
    /// Returns `true` when every evaluated descendant is satisfied,
    /// including the trivial cases of no descendants or every
    /// descendant being skipped by the `types` filter. Short-circuits
    /// on the first unsatisfied descendant.
    ///
    /// Iterates descendants' `requirements.compiled` **directly**
    /// via the split-borrow pattern — no owned snapshot, no
    /// per-descendant `ContractRequirement` clones on the happy
    /// path, and no hash lookup overhead.
    pub fn are_children_satisfied(&mut self, types: Option<&[&str]>) -> bool {
        let root_children = &self.children;
        let cache = &mut self.search_cache;

        Self::check_descendants_satisfied_recursive(root_children, cache, root_children, types)
    }

    /// Recursive worker for [`Self::are_children_satisfied`].
    ///
    /// The `root_*` arguments stay fixed across the whole recursion
    /// — they identify the contract whose children and capabilities
    /// are the full satisfaction scope for every descendant's
    /// requirements. Critically, `root_children` is **not** only
    /// the top-level direct children: the search helpers
    /// ([`Self::any_child_matches_in`] → [`Self::collect_find_children_hashes_walk`])
    /// recurse through `root_children.values()`'s own sub-indexes,
    /// so a requirement in one subtree can be satisfied by a
    /// contract in any other subtree. Keeping `root_*` fixed while
    /// advancing `walk` preserves that cross-subtree visibility.
    ///
    /// The `walk` argument advances to each nested [`ChildrenIndex`]
    /// as we descend, so every descendant is visited exactly once.
    /// A disjoint descendant's own requirements are skipped but
    /// recursion still enters its subtree so its own descendants
    /// still contribute their requirements to the overall check.
    fn check_descendants_satisfied_recursive(
        root_children: &ContractIndex,
        cache: &mut MatcherCache<HashSet<String>>,
        walk: &ContractIndex,
        types: Option<&[&str]>,
    ) -> bool {
        for descendant in walk.values() {
            let evaluate_own = !matches!(
                types,
                Some(allowed) if Self::types_disjoint(allowed, &descendant.requirements.types)
            );
            if evaluate_own {
                for req in descendant.requirements.compiled.values() {
                    if !Self::is_requirement_satisfied_in(root_children, cache, req, types) {
                        return false;
                    }
                }
            }
            if !Self::check_descendants_satisfied_recursive(
                root_children,
                cache,
                &descendant.children,
                types,
            ) {
                return false;
            }
        }
        true
    }

    /// Returns the aggregated list of unsatisfied requirements across
    /// every descendant of `self`.
    ///
    /// # Semantics asymmetry
    ///
    /// **This method does not compose with
    /// [`Self::get_not_satisfied_child_requirements`] on a per-child
    /// basis.** Their disjoint-filter branches diverge deliberately:
    ///
    /// - **Non-disjoint descendant**: the descendant is evaluated
    ///   via the same per-conjunct satisfaction check as
    ///   [`Self::get_not_satisfied_child_requirements`], and the
    ///   unsatisfied subset is appended.
    /// - **Disjoint descendant** (its direct requirement types share
    ///   no element with the `types` filter): the descendant's
    ///   **own** compiled requirements are appended wholesale, **not**
    ///   the result of the per-conjunct check. They target types the
    ///   caller has opted out of, so they cannot possibly be
    ///   satisfied inside the current validation scope. The direct
    ///   requirements alone are enough to signal "this descendant
    ///   has unsatisfied needs" without walking into its subtree.
    ///
    /// In either branch the walk continues into the descendant's
    /// own children, so nested descendants are still processed.
    pub fn get_all_not_satisfied_child_requirements(
        &mut self,
        types: Option<&[&str]>,
    ) -> Vec<ContractRequirement> {
        let root_children = &self.children;
        let cache = &mut self.search_cache;

        let mut out = Vec::new();
        Self::collect_all_not_satisfied_recursive(
            root_children,
            cache,
            root_children,
            types,
            &mut out,
        );
        out
    }

    /// Recursive worker for
    /// [`Self::get_all_not_satisfied_child_requirements`].
    ///
    /// Picks per-descendant between two branches based on the
    /// disjoint-filter check — disjoint descendants contribute their
    /// own compiled requirements wholesale (no per-conjunct check),
    /// non-disjoint descendants contribute only the unsatisfied
    /// subset. In both cases the walk then recurses into the
    /// descendant's children.
    fn collect_all_not_satisfied_recursive(
        root_children: &ContractIndex,
        cache: &mut MatcherCache<HashSet<String>>,
        walk: &ContractIndex,
        types: Option<&[&str]>,
        out: &mut Vec<ContractRequirement>,
    ) {
        for descendant in walk.values() {
            let disjoint = matches!(
                types,
                Some(allowed) if Self::types_disjoint(allowed, &descendant.requirements.types)
            );
            if disjoint {
                out.extend(descendant.requirements.compiled.values().cloned());
            } else {
                for req in descendant.requirements.compiled.values() {
                    if !Self::is_requirement_satisfied_in(root_children, cache, req, types) {
                        out.push(req.clone());
                    }
                }
            }
            Self::collect_all_not_satisfied_recursive(
                root_children,
                cache,
                &descendant.children,
                types,
                out,
            );
        }
    }

    /// Returns `true` when `allowed` and `child_types` share no
    /// element.
    ///
    /// `allowed` is a caller-supplied slice (typically 1–3 entries —
    /// linear scan is cheaper than a `HashSet`); `child_types` is the
    /// owned-string set stored on the requirements index. Iterating
    /// the slice and probing the `HashSet<String>` keeps the
    /// allocation count at zero.
    fn types_disjoint(allowed: &[&str], child_types: &HashSet<String>) -> bool {
        !allowed.iter().any(|t| child_types.contains(*t))
    }

    /// Evaluates a single compiled requirement against the root
    /// described by `children`, caching search results in `cache`.
    ///
    /// Dispatches on the [`ContractRequirement`] variant:
    ///
    /// - [`Match`](ContractRequirement::Match): satisfied when
    ///   [`Self::any_child_matches_in`] reports a match, or when the
    ///   requirement's type is not in `types` (the caller has opted
    ///   out of evaluating this type).
    /// - [`Or`](ContractRequirement::Or): satisfied when at least one
    ///   inner matcher whose type is allowed by `types` has a match,
    ///   or when no inner matcher is of an allowed type (empty
    ///   disjunction after filtering is trivially satisfied).
    /// - [`Not`](ContractRequirement::Not): satisfied when no inner
    ///   matcher whose type is allowed by `types` has a match. An
    ///   empty `Not` is trivially satisfied.
    ///
    /// The `types` filter is consumed as a slice: linear
    /// `slice.contains(&kind)` is cheaper than a `HashSet` probe for
    /// typical filter sizes (1–3 elements) and avoids the per-call
    /// `HashSet` allocation altogether.
    fn is_requirement_satisfied_in(
        children: &ContractIndex,
        cache: &mut MatcherCache<HashSet<String>>,
        req: &ContractRequirement,
        types: Option<&[&str]>,
    ) -> bool {
        let should_evaluate = |kind: &str| -> bool {
            match types {
                Some(slice) => slice.contains(&kind),
                None => true,
            }
        };

        match req {
            ContractRequirement::Match(m) => {
                if !should_evaluate(m.kind.as_str()) {
                    return true;
                }
                Self::any_child_matches_in(children, cache, m)
            }
            ContractRequirement::Or(items) => {
                let mut any_applicable = false;
                for m in items {
                    if !should_evaluate(m.kind.as_str()) {
                        continue;
                    }
                    any_applicable = true;
                    if Self::any_child_matches_in(children, cache, m) {
                        return true;
                    }
                }
                !any_applicable
            }
            ContractRequirement::Not(items) => {
                for m in items {
                    if !should_evaluate(m.kind.as_str()) {
                        continue;
                    }
                    if Self::any_child_matches_in(children, cache, m) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

impl Serialize for Contract {
    /// Serializes a contract by serializing its underlying raw data.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Contract {
    /// Deserializes a contract by first deserializing into a [`RawContract`]
    /// and then constructing through the normal lifecycle (load children,
    /// interpolate, rebuild).
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawContract::deserialize(deserializer)?;
        Ok(Contract::new(raw))
    }
}

impl Identifiable for Contract {
    /// Returns the contract's hash as an owned `String`.
    ///
    /// Triggers lazy computation on first call via [`Contract::hash`].
    fn id(&self) -> String {
        self.hash().to_string()
    }
}

impl PartialEq for Contract {
    /// Two contracts are equal when their deterministic hashes match.
    ///
    /// Triggers lazy hashing on either side if the cache is empty.
    /// Because the hash is a SHA-256 of the serialized raw data,
    /// `a.raw == b.raw` iff `a.hash() == b.hash()`.
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

impl Eq for Contract {}

impl std::hash::Hash for Contract {
    /// Hashes the contract using its deterministic SHA-256 hash string.
    ///
    /// Triggers lazy hashing on the first call to the accessor.
    /// Consistency with [`Eq`] follows because both sides delegate to
    /// the same cached SHA-256.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash().hash(state);
    }
}

impl fmt::Debug for Contract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Contract")
            .field("raw", &self.raw)
            .field("hash", &self.hash)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Deserializes a JSON literal into a [`RawContract`] for testing.
    fn raw(value: Value) -> RawContract {
        serde_json::from_value(value).expect("valid raw contract")
    }

    /// Constructs a [`Contract`] directly from a JSON literal.
    fn contract(value: Value) -> Contract {
        Contract::new(raw(value))
    }

    // ── Construction ─────────────────────────────────────────────────────

    #[test]
    fn constructor_simple_contract() {
        let c = contract(json!({
            "type": "arch.sw",
            "name": "armv7hf",
            "slug": "armv7hf"
        }));

        assert_eq!(c.get_type(), "arch.sw");
        assert_eq!(c.get_slug(), Some("armv7hf"));
        assert_eq!(c.hash().len(), 64, "SHA-256 hex digest is 64 characters");
        assert_eq!(c.children.types().count(), 0);
        assert!(c.requirements.types.is_empty());
        assert!(c.requirements.matchers.is_empty());
        assert!(c.requirements.compiled.is_empty());
    }

    #[test]
    fn constructor_via_deserialize() {
        let c: Contract = serde_json::from_value(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "name": "Debian Wheezy"
        }))
        .expect("deserialize contract");
        assert_eq!(c.get_type(), "sw.os");
        assert_eq!(c.get_slug(), Some("debian"));
        assert_eq!(c.hash().len(), 64);
    }

    #[test]
    fn hash_is_lazy_and_recomputes_after_rebuild() {
        // A fresh contract has an empty OnceLock; the first hash() call
        // populates it. After a mutation that invalidates the cell, the
        // next hash() call must recompute.
        let mut c = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy"
        }));
        let initial = c.hash().to_string();
        // Call again — must return the cached value.
        assert_eq!(c.hash(), initial);

        c.add_child(contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf"
        })));
        // add_child -> rebuild -> invalidate_hash; the next hash() call
        // should produce a different digest.
        assert_ne!(c.hash(), initial);
    }

    #[test]
    fn hash_clone_mutation_does_not_affect_original() {
        // `OnceLock::clone` copies the current state of the cell, so a
        // clone of an already-hashed contract starts life with a
        // populated cell. The invariant we need to pin is that a
        // subsequent mutation on the clone does *not* leak back into
        // the original: `rebuild` on the clone must replace the
        // clone's cell without touching the original's, and both
        // contracts must end up with hashes consistent with their own
        // `raw` data.
        let original = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy"
        }));
        let original_hash = original.hash().to_string();

        let mut clone = original.clone();
        // Clone carries the already-computed hash forward.
        assert_eq!(clone.hash(), original_hash);

        clone.add_child(contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf"
        })));

        // Original is untouched.
        assert_eq!(original.hash(), original_hash);
        // Clone recomputed to a new digest.
        assert_ne!(clone.hash(), original_hash);
    }

    // ── Accessors ─────────────────────────────────────────────────────────

    #[test]
    fn get_type_returns_type() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        assert_eq!(c.get_type(), "arch.sw");
    }

    #[test]
    fn get_slug_returns_slug() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        assert_eq!(c.get_slug(), Some("armv7hf"));
    }

    #[test]
    fn get_version_returns_version() {
        let c = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "name": "Debian Wheezy"
        }));
        assert_eq!(c.get_version().as_deref(), Some("wheezy"));
    }

    #[test]
    fn get_version_returns_none_when_absent() {
        let c = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "name": "Debian"
        }));
        assert_eq!(c.get_version(), None);
    }

    #[test]
    fn get_canonical_slug_falls_back_to_slug() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        assert_eq!(c.get_canonical_slug(), Some("armv7hf"));
    }

    #[test]
    fn get_canonical_slug_returns_canonical_slug_when_present() {
        let c = contract(json!({
            "type": "hw.device-type",
            "slug": "rpi",
            "canonicalSlug": "raspberrypi"
        }));
        assert_eq!(c.get_canonical_slug(), Some("raspberrypi"));
    }

    #[test]
    fn get_reference_string_without_version() {
        let c = contract(json!({
            "type": "sw.arch",
            "slug": "armv7hf",
            "name": "ARMV7HF"
        }));
        assert_eq!(c.get_reference_string(), "armv7hf");
    }

    #[test]
    fn get_reference_string_with_version() {
        let c = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "name": "Debian Wheezy"
        }));
        assert_eq!(c.get_reference_string(), "debian@wheezy");
    }

    #[test]
    fn get_all_slugs_without_aliases() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        let slugs: HashSet<&str> = c.get_all_slugs().collect();
        assert_eq!(slugs.len(), 1);
        assert!(slugs.contains("armv7hf"));
    }

    #[test]
    fn get_all_slugs_with_aliases() {
        let c = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi",
            "name": "Raspberry Pi",
            "aliases": ["rpi", "raspberry-pi"]
        }));
        let slugs: HashSet<&str> = c.get_all_slugs().collect();
        assert_eq!(slugs.len(), 3);
        assert!(slugs.contains("raspberrypi"));
        assert!(slugs.contains("rpi"));
        assert!(slugs.contains("raspberry-pi"));
    }

    #[test]
    fn get_all_slugs_with_empty_aliases() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf",
            "aliases": []
        }));
        let slugs: HashSet<&str> = c.get_all_slugs().collect();
        assert_eq!(slugs.len(), 1);
        assert!(slugs.contains("armv7hf"));
    }

    #[test]
    fn has_aliases_false_when_absent() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        assert!(!c.has_aliases());
    }

    #[test]
    fn has_aliases_true_when_present() {
        let c = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi",
            "name": "Raspberry Pi",
            "aliases": ["rpi", "raspberry-pi"]
        }));
        assert!(c.has_aliases());
    }

    #[test]
    fn has_aliases_false_when_empty_array() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf",
            "aliases": []
        }));
        assert!(!c.has_aliases());
    }

    // ── Hash ──────────────────────────────────────────────────────────────

    #[test]
    fn hash_equal_contracts_have_equal_hashes() {
        let a = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        let b = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn hash_different_contracts_have_different_hashes() {
        let a = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        let b = contract(json!({
            "type": "arch.sw",
            "slug": "i386",
            "name": "i386"
        }));
        assert_ne!(a.hash(), b.hash());
    }

    #[test]
    fn rebuild_after_mutation_yields_new_hash() {
        // Direct raw mutation followed by rebuild must invalidate the
        // cached hash so the next read reflects the change.
        let mut c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        let original = c.hash().to_string();
        c.raw.body.name = Some("ARM v7".to_string());
        c.rebuild();
        assert_ne!(c.hash(), original);
    }

    // ── Templates / Interpolate ──────────────────────────────────────────

    #[test]
    fn interpolate_resolves_templates_on_construction() {
        let c = contract(json!({
            "type": "arch.sw",
            "version": "7",
            "name": "ARM v{{this.version}}",
            "slug": "armv7hf"
        }));
        assert_eq!(c.raw.body.name.as_deref(), Some("ARM v7"));
    }

    #[test]
    fn interpolate_leaves_unresolved_templates_intact() {
        let c = contract(json!({
            "type": "arch.sw",
            "name": "{{this.displayName}}",
            "slug": "armv7hf"
        }));
        assert_eq!(c.raw.body.name.as_deref(), Some("{{this.displayName}}"));
    }

    #[test]
    fn interpolate_resolves_templates_after_mutation() {
        let mut c = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "name": "Debian {{this.data.codename}}",
            "data": {
                "url": "https://example.org/{{this.type}}/{{this.slug}}/{{this.version}}.tar.gz"
            }
        }));

        assert_eq!(
            c.raw.body.data.as_ref().unwrap()["url"],
            json!("https://example.org/sw.os/debian/wheezy.tar.gz")
        );
        c.raw
            .body
            .data
            .as_mut()
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("codename".to_string(), json!("Wheezy"));
        c.interpolate();

        assert_eq!(c.raw.body.name.as_deref(), Some("Debian Wheezy"));
    }

    #[test]
    fn interpolate_does_not_template_children() {
        let c = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "children": {
                "foo": {
                    "bar": {
                        "slug": "{{this.version}}-child",
                        "type": "foo.bar"
                    }
                }
            }
        }));

        // Children are interpolated against their own fields; the child has
        // no `version`, so the template stays unresolved.
        let tree = c.raw.body.children.as_ref().unwrap();
        let extracted = children_tree::get_all(tree);
        assert_eq!(extracted.len(), 1);
        assert_eq!(
            extracted[0].body.slug.as_ref().unwrap().as_str(),
            "{{this.version}}-child"
        );
    }

    #[test]
    fn interpolate_keeps_hash_stable_when_raw_unchanged() {
        let mut c = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "name": "Debian"
        }));
        let original = c.hash().to_string();
        c.interpolate();
        assert_eq!(c.hash(), original);
    }

    // ── Serialize (replaces the old `to_json` helper) ───────────────────

    #[test]
    fn serialize_round_trip_without_children() {
        let source = json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "name": "Debian Wheezy"
        });
        let c = contract(source.clone());
        assert_eq!(serde_json::to_value(&c).unwrap(), source);
    }

    #[test]
    fn serialize_round_trip_with_single_child() {
        let source = json!({
            "type": "misc.collection",
            "slug": "my-collection",
            "children": {
                "arch": {
                    "sw": {
                        "type": "arch.sw",
                        "name": "armv7hf",
                        "slug": "armv7hf"
                    }
                }
            }
        });
        let c = contract(source.clone());
        assert_eq!(serde_json::to_value(&c).unwrap(), source);
    }

    #[test]
    fn serialize_round_trip_with_two_children_same_type() {
        let source = json!({
            "type": "misc.collection",
            "slug": "my-collection",
            "children": {
                "arch": {
                    "sw": {
                        "armv7hf": {
                            "type": "arch.sw",
                            "name": "armv7hf",
                            "slug": "armv7hf"
                        },
                        "armel": {
                            "type": "arch.sw",
                            "name": "armel",
                            "slug": "armel"
                        }
                    }
                }
            }
        });
        let c = contract(source.clone());
        assert_eq!(serde_json::to_value(&c).unwrap(), source);
    }

    // ── Equality (PartialEq / Eq) ────────────────────────────────────────

    #[test]
    fn eq_true_for_equal_contracts() {
        let a = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        let b = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        assert_eq!(a, b);
    }

    #[test]
    fn eq_false_for_different_contracts() {
        let a = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        let b = contract(json!({
            "type": "arch.sw",
            "slug": "i386",
            "name": "i386"
        }));
        assert_ne!(a, b);
    }

    // ── Requirements ─────────────────────────────────────────────────────

    #[test]
    fn requirements_empty_when_requires_is_empty() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf",
            "requires": []
        }));
        assert!(c.requirements.types.is_empty());
        assert!(c.requirements.matchers.is_empty());
        assert!(c.requirements.compiled.is_empty());
    }

    #[test]
    fn requirements_simple_match() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf",
            "requires": [
                {"type": "hw.device-type", "slug": "raspberry-pi"}
            ]
        }));
        assert_eq!(c.requirements.types.len(), 1);
        assert!(c.requirements.types.contains("hw.device-type"));
        assert_eq!(c.requirements.matchers.len(), 1);
        assert_eq!(c.requirements.matchers["hw.device-type"].len(), 1);
        assert_eq!(c.requirements.compiled.len(), 1);

        // The typed matcher stored in the index is the same
        // `ContractMatcher` that was deserialized from `requires`
        // — no Contract wrapping, no extra fields.
        let matcher = c.requirements.matchers["hw.device-type"]
            .values()
            .next()
            .unwrap();
        assert_eq!(matcher.kind.as_str(), "hw.device-type");
        assert_eq!(matcher.slug.as_ref().unwrap().as_str(), "raspberry-pi");
        assert!(matcher.version.is_none());
        assert!(matcher.data.is_none());

        // The compiled requirement for a simple `requires` entry is
        // the `Match` variant carrying the same matcher.
        let compiled = c.requirements.compiled.values().next().unwrap();
        match compiled {
            ContractRequirement::Match(m) => {
                assert_eq!(m.kind.as_str(), "hw.device-type");
                assert_eq!(m.slug.as_ref().unwrap().as_str(), "raspberry-pi");
            }
            other => panic!("expected Match requirement, got {other:?}"),
        }
    }

    #[test]
    fn requirements_duplicate_matchers_deduplicated() {
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf",
            "requires": [
                {"type": "hw.device-type", "slug": "raspberry-pi"},
                {"type": "hw.device-type", "slug": "raspberry-pi"}
            ]
        }));
        assert_eq!(
            c.requirements.matchers["hw.device-type"].len(),
            1,
            "matchers by type are deduplicated"
        );
        // Compiled requirements deduplicate on the same Identifiable
        // key (ContractRequirement::Match of an identical matcher).
        assert_eq!(c.requirements.compiled.len(), 1);
    }

    #[test]
    fn requirements_or_operation() {
        let c = contract(json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [
                {
                    "or": [
                        {"type": "hw.device-type", "slug": "raspberry-pi"},
                        {"type": "hw.device-type", "slug": "raspberry-pi2"}
                    ]
                }
            ]
        }));
        assert!(c.requirements.types.contains("hw.device-type"));
        assert_eq!(
            c.requirements.matchers["hw.device-type"].len(),
            2,
            "both disjuncts registered as per-type matchers"
        );
        assert_eq!(
            c.requirements.compiled.len(),
            1,
            "one top-level operation requirement compiled"
        );

        // The compiled requirement preserves the `Or` variant — no
        // conversion to a Contract wrapper with an `operation` tag.
        let compiled = c.requirements.compiled.values().next().unwrap();
        match compiled {
            ContractRequirement::Or(items) => {
                assert_eq!(items.len(), 2);
                let slugs: HashSet<&str> = items
                    .iter()
                    .map(|m| m.slug.as_ref().unwrap().as_str())
                    .collect();
                assert!(slugs.contains("raspberry-pi"));
                assert!(slugs.contains("raspberry-pi2"));
            }
            other => panic!("expected Or requirement, got {other:?}"),
        }
    }

    #[test]
    fn requirements_not_operation() {
        let c = contract(json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [
                {"not": [{"type": "sw.os", "slug": "windows"}]}
            ]
        }));
        assert!(c.requirements.types.contains("sw.os"));
        assert_eq!(c.requirements.compiled.len(), 1);

        let compiled = c.requirements.compiled.values().next().unwrap();
        match compiled {
            ContractRequirement::Not(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].kind.as_str(), "sw.os");
                assert_eq!(items[0].slug.as_ref().unwrap().as_str(), "windows");
            }
            other => panic!("expected Not requirement, got {other:?}"),
        }
    }

    #[test]
    fn requirement_types_reflects_own_requires_entries() {
        let c = contract(json!({
            "type": "sw.stack",
            "slug": "nodejs",
            "requires": [
                {"type": "hw.device-type", "slug": "raspberry-pi"},
                {"type": "arch.sw", "slug": "armv7hf"},
                {"or": [
                    {"type": "sw.os", "slug": "debian"},
                    {"type": "sw.os", "slug": "fedora"}
                ]}
            ]
        }));

        let types: HashSet<&str> = c.requirement_types().collect();
        assert_eq!(types, HashSet::from(["hw.device-type", "arch.sw", "sw.os"]));
    }

    #[test]
    fn requirement_matchers_for_type_returns_registered_matchers() {
        let c = contract(json!({
            "type": "sw.stack",
            "slug": "nodejs",
            "requires": [
                {"type": "hw.device-type", "slug": "raspberry-pi"},
                {"type": "hw.device-type", "slug": "raspberry-pi2"},
                {"type": "arch.sw", "slug": "armv7hf"}
            ]
        }));

        let device_slugs: HashSet<&str> = c
            .requirement_matchers_for_type("hw.device-type")
            .map(|m| m.slug.as_ref().unwrap().as_str())
            .collect();
        assert_eq!(
            device_slugs,
            HashSet::from(["raspberry-pi", "raspberry-pi2"])
        );

        let arch: Vec<&ContractMatcher> = c.requirement_matchers_for_type("arch.sw").collect();
        assert_eq!(arch.len(), 1);
        assert_eq!(arch[0].kind.as_str(), "arch.sw");
        assert_eq!(arch[0].slug.as_ref().unwrap().as_str(), "armv7hf");

        // Unknown requirement type yields an empty iterator.
        assert_eq!(
            c.requirement_matchers_for_type("sw.os").count(),
            0,
            "unknown requirement type returns empty iterator"
        );
    }

    // ── build ────────────────────────────────────────────────────────────

    #[test]
    fn build_single_contract_no_variants() {
        let contracts = Contract::build(&raw(json!({
            "name": "Debian Wheezy",
            "slug": "debian",
            "version": "wheezy",
            "type": "sw.os"
        })));
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].get_slug(), Some("debian"));
        assert_eq!(contracts[0].get_version().as_deref(), Some("wheezy"));
    }

    #[test]
    fn build_expands_templates() {
        let contracts = Contract::build(&raw(json!({
            "name": "Debian {{this.data.codename}}",
            "slug": "debian",
            "version": "wheezy",
            "type": "sw.os",
            "data": {
                "codename": "Wheezy",
                "url": "https://example.org/{{this.type}}/{{this.slug}}/{{this.version}}.tar.gz"
            }
        })));
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].raw.body.name.as_deref(), Some("Debian Wheezy"));
        assert_eq!(
            contracts[0].raw.body.data.as_ref().unwrap()["url"],
            json!("https://example.org/sw.os/debian/wheezy.tar.gz")
        );
    }

    #[test]
    fn build_supports_slug_and_type_templates() {
        let contracts = Contract::build(&raw(json!({
            "name": "Debian Wheezy",
            "slug": "{{this.data.slug}}",
            "version": "wheezy",
            "type": "{{this.data.type}}",
            "data": {"slug": "debian", "type": "sw.os"}
        })));
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].get_slug(), Some("debian"));
        assert_eq!(contracts[0].get_type(), "sw.os");
    }

    #[test]
    fn build_expands_variants() {
        let contracts = Contract::build(&raw(json!({
            "slug": "debian",
            "type": "sw.os",
            "variants": [
                {"version": "wheezy"},
                {"version": "jessie"},
                {"version": "sid"}
            ]
        })));
        assert_eq!(contracts.len(), 3);
        let versions: Vec<_> = contracts.iter().map(|c| c.get_version()).collect();
        assert_eq!(
            versions,
            vec![
                Some("wheezy".to_string()),
                Some("jessie".to_string()),
                Some("sid".to_string())
            ]
        );
    }

    #[test]
    fn build_variants_with_templates() {
        let contracts = Contract::build(&raw(json!({
            "name": "debian {{this.version}}",
            "slug": "debian",
            "type": "sw.os",
            "variants": [
                {"version": "wheezy"},
                {"version": "jessie"}
            ]
        })));
        assert_eq!(contracts.len(), 2);
        assert_eq!(contracts[0].raw.body.name.as_deref(), Some("debian wheezy"));
        assert_eq!(contracts[1].raw.body.name.as_deref(), Some("debian jessie"));
    }

    #[test]
    fn build_expands_aliases() {
        let contracts = Contract::build(&raw(json!({
            "slug": "debian",
            "type": "sw.os",
            "version": "jessie",
            "aliases": ["foo", "bar"]
        })));
        assert_eq!(contracts.len(), 3);
        // Aliases come first, base contract last.
        assert_eq!(contracts[0].get_slug(), Some("foo"));
        assert_eq!(contracts[0].get_canonical_slug(), Some("debian"));
        assert_eq!(contracts[1].get_slug(), Some("bar"));
        assert_eq!(contracts[1].get_canonical_slug(), Some("debian"));
        assert_eq!(contracts[2].get_slug(), Some("debian"));
        assert!(!contracts[2].has_aliases());
    }

    #[test]
    fn build_variants_and_aliases() {
        let contracts = Contract::build(&raw(json!({
            "name": "debian {{this.version}}",
            "slug": "debian",
            "type": "sw.os",
            "variants": [
                {"version": "wheezy"},
                {"version": "jessie"}
            ],
            "aliases": ["foo", "bar"]
        })));
        assert_eq!(contracts.len(), 6);

        let slugs: Vec<_> = contracts
            .iter()
            .map(|c| c.get_slug().unwrap().to_string())
            .collect();
        assert_eq!(slugs, vec!["foo", "bar", "debian", "foo", "bar", "debian"]);

        let versions: Vec<_> = contracts.iter().map(|c| c.get_version().unwrap()).collect();
        assert_eq!(
            versions,
            vec!["wheezy", "wheezy", "wheezy", "jessie", "jessie", "jessie"]
        );
    }

    // ── round-trip ───────────────────────────────────────────────────────

    #[test]
    fn constructor_round_trip_preserves_raw() {
        let source = json!({
            "type": "misc.collection",
            "slug": "my-collection",
            "children": {
                "arch": {
                    "sw": {
                        "type": "arch.sw",
                        "name": "armv7hf",
                        "slug": "armv7hf"
                    }
                }
            }
        });
        let c = contract(source);
        let via_json = serde_json::to_value(&c).unwrap();
        let reconstructed: Contract = serde_json::from_value(via_json).unwrap();
        assert_eq!(c, reconstructed);
    }

    // ── Coverage gaps surfaced by code review ────────────────────────────

    #[test]
    fn requirements_nested_or_inside_or_fails_to_deserialize() {
        // Nested boolean operations are rejected at the type level:
        // `Or` / `Not` carry `Vec<ContractMatcher>`, so an inner
        // `{"or": [...]}` has no `type` field and fails as a matcher.
        let result: Result<RawContract, _> = serde_json::from_value(json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [
                { "or": [
                    { "or": [
                        { "type": "hw.device-type", "slug": "rpi" }
                    ]}
                ]}
            ]
        }));
        assert!(result.is_err());
    }

    #[test]
    fn requirements_nested_not_inside_or_fails_to_deserialize() {
        let result: Result<RawContract, _> = serde_json::from_value(json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [
                { "or": [
                    { "type": "hw.device-type", "slug": "rpi" },
                    { "not": [{ "type": "sw.os", "slug": "windows" }] }
                ]}
            ]
        }));
        assert!(result.is_err());
    }

    #[test]
    fn identifiable_id_returns_hash_string() {
        // Identifiable::id is a thin wrapper over Contract::hash.
        let c = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        let id = Identifiable::id(&c);
        assert_eq!(id.len(), 64, "SHA-256 hex digest is 64 characters");
        assert_eq!(id, c.hash());
    }

    #[test]
    // Contract contains a `OnceLock<String>` for lazy hashing, which
    // is interior mutability. Clippy flags types with interior
    // mutability as unsafe `HashSet` keys because a mutation could
    // change the key's hash out from under the set. For `Contract`
    // this is safe: the cell is only ever populated from empty (never
    // overwritten with a different value), and the computed hash is a
    // deterministic SHA-256 of the raw contract data — `Hash` and
    // `PartialEq` both delegate to that cached string, so equal keys
    // always produce equal hashes.
    #[allow(clippy::mutable_key_type)]
    fn hash_impl_is_consistent_with_eq_for_hashed_contracts() {
        use std::collections::HashSet;

        let a = contract(json!({
            "type": "arch.sw",
            "slug": "armv7hf",
            "name": "armv7hf"
        }));
        let a_clone = a.clone();
        let b = contract(json!({
            "type": "arch.sw",
            "slug": "i386",
            "name": "i386"
        }));

        let mut set = HashSet::new();
        set.insert(a);
        // Inserting an equal contract must be deduplicated by the set.
        assert!(!set.insert(a_clone));
        assert!(set.insert(b));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn get_reference_string_without_slug_or_version() {
        // Deserialization accepts a contract with only a type; get_slug
        // returns None. The reference string degrades to an empty string.
        let c = contract(json!({ "type": "arch.sw" }));
        assert_eq!(c.get_slug(), None);
        assert_eq!(c.get_reference_string(), "");
    }

    #[test]
    fn get_reference_string_without_slug_but_with_version() {
        let c = contract(json!({
            "type": "arch.sw",
            "version": "1.0.0"
        }));
        assert_eq!(c.get_slug(), None);
        // Current behavior: "@1.0.0" — documented degradation for the
        // no-slug case. Pinned here so future changes surface explicitly.
        assert_eq!(c.get_reference_string(), "@1.0.0");
    }

    #[test]
    fn interpolate_preserves_and_templates_extra_fields() {
        // Unknown top-level keys land in `RawContract::extra` via
        // serde(flatten). The interpolate round-trip must preserve them
        // and apply template substitution inside their string values.
        let c = contract(json!({
            "type": "hw.device-type",
            "slug": "rpi",
            "custom_literal": "static-value",
            "custom_templated": "arch={{this.slug}}",
            "nested": { "inner": "t={{this.type}}" }
        }));

        assert_eq!(
            c.raw.extra.get("custom_literal"),
            Some(&json!("static-value"))
        );
        assert_eq!(
            c.raw.extra.get("custom_templated"),
            Some(&json!("arch=rpi"))
        );
        assert_eq!(
            c.raw.extra.get("nested"),
            Some(&json!({ "inner": "t=hw.device-type" }))
        );
    }

    #[test]
    fn rebuild_clears_children_tree_when_index_empties() {
        // Construct a contract with one child, then manually empty the
        // index and rerun rebuild. The raw.children tree must also clear.
        let mut c = contract(json!({
            "type": "misc.collection",
            "slug": "my-collection",
            "children": {
                "arch": {
                    "sw": {
                        "type": "arch.sw",
                        "slug": "armv7hf",
                        "name": "armv7hf"
                    }
                }
            }
        }));
        assert!(c.raw.body.children.is_some());

        c.children = ContractIndex::default();
        c.rebuild();

        assert!(
            c.raw.body.children.is_none(),
            "empty children index must clear raw.body.children"
        );
    }

    // ── children management ──────────────────────────────────────────────

    /// Minimal `sw.os` contract helper (slug + version).
    fn sw_os(slug: &str, version: &str) -> Contract {
        contract(json!({
            "type": "sw.os",
            "slug": slug,
            "version": version,
            "name": format!("{slug} {version}"),
        }))
    }

    /// Minimal `sw.blob` contract helper.
    fn sw_blob(slug: &str, version: &str) -> Contract {
        contract(json!({
            "type": "sw.blob",
            "slug": slug,
            "version": version,
            "name": slug,
        }))
    }

    /// Empty container contract used as the root of tree-shape
    /// children tests.
    fn container() -> Contract {
        contract(json!({ "type": "foo", "slug": "bar" }))
    }

    /// Collects a list of Contract references into a HashSet keyed by
    /// the contract hash, so tests can assert set membership without
    /// relying on HashMap iteration order.
    fn hash_set<'a>(contracts: &[&'a Contract]) -> HashSet<&'a str> {
        contracts.iter().map(|c| c.hash()).collect()
    }

    // add_child

    #[test]
    fn add_child_registers_contract_in_all_indexes() {
        let mut parent = container();
        let child = sw_os("debian", "wheezy");
        let child_hash = child.hash().to_string();

        parent.add_child(child);

        assert_eq!(parent.children.values().count(), 1);
        assert!(parent.children.get(&child_hash).is_some());
        let types: HashSet<&str> = parent.children.types().collect();
        assert_eq!(types, HashSet::from(["sw.os"]));
    }

    #[test]
    fn add_child_two_different_types() {
        let mut parent = container();
        parent.add_child(sw_os("debian", "wheezy"));
        parent.add_child(sw_blob("nodejs", "4.8.0"));

        assert_eq!(parent.children.values().count(), 2);
        let types: HashSet<&str> = parent.children.types().collect();
        assert_eq!(types, HashSet::from(["sw.os", "sw.blob"]));
    }

    #[test]
    fn add_child_dedupes_on_hash() {
        let mut parent = container();
        let child = sw_os("debian", "wheezy");
        parent.add_child(child.clone());
        let first_hash = parent.hash().to_string();
        parent.add_child(child);

        assert_eq!(parent.children.values().count(), 1);
        assert_eq!(parent.hash(), first_hash);
    }

    #[test]
    fn add_child_same_type_different_slug() {
        let mut parent = container();
        parent.add_child(sw_os("debian", "wheezy"));
        parent.add_child(sw_os("fedora", "25"));

        assert_eq!(parent.children.values().count(), 2);
        let types: HashSet<&str> = parent.children.types().collect();
        assert_eq!(types, HashSet::from(["sw.os"]));
    }

    #[test]
    fn add_child_same_slug_different_versions() {
        let mut parent = container();
        parent.add_child(sw_os("debian", "wheezy"));
        parent.add_child(sw_os("debian", "jessie"));

        assert_eq!(parent.children.values().count(), 2);
    }

    #[test]
    fn add_child_rehashes_parent_by_default() {
        let mut parent = container();
        let original = parent.hash().to_string();
        parent.add_child(sw_os("debian", "wheezy"));
        assert_ne!(parent.hash(), original);
    }

    #[test]
    fn add_child_serializes_single_child_tree() {
        let mut parent = container();
        parent.add_child(sw_os("debian", "wheezy"));
        let tree = serde_json::to_value(&parent).unwrap();
        assert_eq!(
            tree["children"],
            json!({
                "sw": {
                    "os": {
                        "type": "sw.os",
                        "slug": "debian",
                        "version": "wheezy",
                        "name": "debian wheezy"
                    }
                }
            })
        );
    }

    // add_children

    #[test]
    fn add_children_adds_multiple() {
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_os("debian", "jessie")]);
        assert_eq!(parent.children.values().count(), 2);
    }

    #[test]
    fn add_children_dedupes_duplicates_in_batch() {
        let mut parent = container();
        let child = sw_os("debian", "wheezy");
        parent.add_children(vec![child.clone(), child.clone(), child]);
        assert_eq!(parent.children.values().count(), 1);
    }

    #[test]
    fn add_children_empty_batch_is_noop_but_still_rebuilds() {
        let mut parent = container();
        let original = parent.hash().to_string();
        parent.add_children(Vec::<Contract>::new());
        assert_eq!(parent.hash(), original);
        assert!(parent.children.is_empty());
    }

    #[test]
    fn add_children_is_insertion_order_independent_for_same_slug() {
        let mut a = container();
        a.add_children(vec![sw_os("debian", "wheezy"), sw_os("debian", "jessie")]);
        let mut b = container();
        b.add_children(vec![sw_os("debian", "jessie"), sw_os("debian", "wheezy")]);
        assert_eq!(a, b, "sort in children_tree::build gives stable hashes");
    }

    #[test]
    fn add_child_duplicate_leaves_hash_cell_empty() {
        // `ChildrenIndex::insert` returns false on duplicates, and
        // `add_child` uses the return value to skip `rebuild` — so a
        // duplicate insertion must not invalidate the already-computed
        // hash. We prove it by observing the cached hash stays stable
        // across a duplicate add.
        let mut parent = container();
        parent.add_child(sw_os("debian", "wheezy"));
        let before = parent.hash().to_string();
        parent.add_child(sw_os("debian", "wheezy"));
        assert_eq!(parent.hash(), before);
    }

    #[test]
    fn add_children_all_duplicates_skips_rebuild() {
        let mut parent = container();
        parent.add_child(sw_os("debian", "wheezy"));
        let before = parent.hash().to_string();
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_os("debian", "wheezy")]);
        assert_eq!(parent.hash(), before);
    }

    #[test]
    fn add_children_mixed_batch_rebuilds_once() {
        let mut parent = container();
        parent.add_child(sw_os("debian", "wheezy"));
        let before = parent.hash().to_string();
        // One duplicate, one new contract — rebuild should still run
        // exactly once at the end (observable only by the hash
        // changing to reflect the new child).
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_os("fedora", "25")]);
        assert_ne!(parent.hash(), before);
        assert_eq!(parent.children.values().count(), 2);
    }

    // remove_child

    #[test]
    fn remove_child_removes_matching_contract() {
        let mut parent = container();
        let c1 = sw_os("debian", "wheezy");
        let c2 = sw_os("debian", "jessie");
        let c3 = sw_os("fedora", "25");
        parent.add_children(vec![c1.clone(), c2.clone(), c3.clone()]);
        parent.remove_child(&c2);

        let mut expected = container();
        expected.add_children(vec![c1, c3]);
        assert_eq!(parent, expected);
    }

    #[test]
    fn remove_child_ignores_unknown_contract() {
        let mut parent = container();
        let c1 = sw_os("debian", "wheezy");
        parent.add_child(c1.clone());
        let before = parent.hash().to_string();
        parent.remove_child(&sw_blob("nodejs", "4.8.0"));
        assert_eq!(parent.hash(), before);
    }

    #[test]
    fn remove_child_cleans_up_empty_slug_entry() {
        // When removing the last child of a slug, the slug entry should
        // be removed from by_type_slug. Verified indirectly via round-trip
        // equality with a container that was never told about wheezy.
        let mut parent = container();
        let wheezy = sw_os("debian", "wheezy");
        let fedora = sw_os("fedora", "25");
        parent.add_children(vec![wheezy.clone(), fedora.clone()]);
        parent.remove_child(&wheezy);

        let mut expected = container();
        expected.add_child(fedora);
        assert_eq!(parent, expected);
    }

    #[test]
    fn remove_child_cleans_up_empty_type_entry() {
        let mut parent = container();
        let os = sw_os("debian", "wheezy");
        let blob = sw_blob("nodejs", "4.8.0");
        parent.add_children(vec![os.clone(), blob.clone()]);
        parent.remove_child(&os);

        let mut expected = container();
        expected.add_child(blob);
        assert_eq!(parent, expected);

        let remaining_types: HashSet<&str> = parent.children.types().collect();
        assert_eq!(remaining_types, HashSet::from(["sw.blob"]));
    }

    #[test]
    fn remove_child_with_aliases() {
        let mut parent = container();
        let rpi = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi",
            "name": "Raspberry Pi",
            "aliases": ["rpi", "raspberry-pi"]
        }));
        let blob = sw_blob("nodejs", "4.8.0");
        parent.add_children(vec![rpi.clone(), blob.clone()]);
        parent.remove_child(&rpi);

        let mut expected = container();
        expected.add_child(blob);
        assert_eq!(parent, expected);
    }

    #[test]
    fn remove_child_with_aliases_keeps_sibling_aliased_contract() {
        let mut parent = container();
        let nuc = contract(json!({
            "type": "hw.device-type",
            "slug": "intel-nuc",
            "name": "Intel NUC",
            "aliases": ["nuc"]
        }));
        let rpi = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi",
            "name": "Raspberry Pi",
            "aliases": ["rpi", "raspberry-pi"]
        }));
        parent.add_children(vec![nuc.clone(), rpi.clone()]);
        parent.remove_child(&rpi);

        let mut expected = container();
        expected.add_child(nuc);
        assert_eq!(parent, expected);
    }

    #[test]
    fn remove_child_rehashes_parent_by_default() {
        let mut parent = container();
        let c1 = sw_os("debian", "wheezy");
        let c2 = sw_os("debian", "jessie");
        parent.add_children(vec![c1.clone(), c2]);
        let before = parent.hash().to_string();
        parent.remove_child(&c1);
        assert_ne!(parent.hash(), before);
    }

    #[test]
    fn remove_child_last_child_clears_children_tree() {
        let mut parent = container();
        let c = sw_os("debian", "wheezy");
        parent.add_child(c.clone());
        parent.remove_child(&c);
        assert!(parent.raw.body.children.is_none());
        assert!(parent.children.is_empty());
    }

    // get_child_by_hash

    #[test]
    fn get_child_by_hash_returns_direct_child() {
        let mut parent = container();
        let child = sw_os("debian", "wheezy");
        let child_hash = child.hash().to_string();
        parent.add_child(child.clone());

        let found = parent.get_child_by_hash(&child_hash).unwrap();
        assert_eq!(found, &child);
    }

    #[test]
    fn get_child_by_hash_returns_none_for_unknown_hash() {
        let parent = container();
        assert!(parent.get_child_by_hash("nonexistent").is_none());
    }

    #[test]
    fn get_child_by_hash_is_non_recursive() {
        // Only direct children are looked up — nested children must be
        // reached via their parent's own get_child_by_hash.
        let mut grandchild_parent = sw_os("debian", "wheezy");
        let grandchild = sw_blob("nodejs", "4.8.0");
        let grandchild_hash = grandchild.hash().to_string();
        grandchild_parent.add_child(grandchild);

        let mut root = container();
        root.add_child(grandchild_parent);

        assert!(root.get_child_by_hash(&grandchild_hash).is_none());
    }

    // get_children

    #[test]
    fn get_children_empty() {
        let parent = container();
        assert!(parent.get_children().is_empty());
    }

    #[test]
    fn get_children_single_child() {
        let mut parent = container();
        let child = sw_os("debian", "wheezy");
        parent.add_child(child.clone());
        let result = parent.get_children();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], &child);
    }

    #[test]
    fn get_children_multiple_different_slugs() {
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        let b = sw_os("fedora", "25");
        parent.add_children(vec![a.clone(), b.clone()]);
        let children = parent.get_children();
        assert_eq!(children.len(), 2);
        assert_eq!(hash_set(&children), hash_set(&[&a, &b]));
    }

    #[test]
    fn get_children_multiple_same_slug() {
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        let b = sw_os("debian", "jessie");
        parent.add_children(vec![a.clone(), b.clone()]);
        let children = parent.get_children();
        assert_eq!(children.len(), 2);
        assert_eq!(hash_set(&children), hash_set(&[&a, &b]));
    }

    #[test]
    fn get_children_filtered_by_one_type() {
        let mut parent = container();
        let os = sw_os("debian", "wheezy");
        let blob = sw_blob("nodejs", "4.8.0");
        parent.add_children(vec![os.clone(), blob]);
        let result = parent.get_children_filtered(&["sw.os"]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], &os);
    }

    #[test]
    fn get_children_filtered_by_two_types() {
        let mut parent = container();
        let os = sw_os("debian", "wheezy");
        let blob = sw_blob("nodejs", "4.8.0");
        let dt = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi",
            "name": "Raspberry Pi"
        }));
        parent.add_children(vec![os.clone(), blob.clone(), dt]);
        let result = parent.get_children_filtered(&["sw.os", "sw.blob"]);
        assert_eq!(result.len(), 2);
        assert_eq!(hash_set(&result), hash_set(&[&os, &blob]));
    }

    #[test]
    fn get_children_filtered_ignores_unknown_types() {
        let mut parent = container();
        let os = sw_os("debian", "wheezy");
        let blob = sw_blob("nodejs", "4.8.0");
        parent.add_children(vec![os.clone(), blob]);
        let result = parent.get_children_filtered(&["sw.os", "hello"]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], &os);
    }

    #[test]
    fn get_children_filtered_empty_when_no_match() {
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_blob("nodejs", "4.8.0")]);
        assert!(parent.get_children_filtered(&["hello", "world"]).is_empty());
    }

    #[test]
    fn get_children_returns_each_aliased_contract_once() {
        let mut parent = container();
        let blob = sw_blob("nodejs", "4.8.0");
        let rpi = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi",
            "name": "Raspberry Pi",
            "aliases": ["rpi", "raspberry-pi"]
        }));
        parent.add_children(vec![blob.clone(), rpi.clone()]);
        let result = parent.get_children();
        assert_eq!(result.len(), 2);
        assert_eq!(hash_set(&result), hash_set(&[&blob, &rpi]));
    }

    #[test]
    fn get_children_recurses_into_nested() {
        let mut inner = sw_os("debian", "wheezy");
        let grand = sw_blob("nodejs", "4.8.0");
        inner.add_child(grand.clone());

        let mut parent = container();
        parent.add_child(inner.clone());

        let result = parent.get_children();
        assert_eq!(result.len(), 2);
        let hashes: HashSet<&str> = result.iter().map(|c| c.hash()).collect();
        assert!(hashes.contains(inner.hash()));
        assert!(hashes.contains(grand.hash()));
    }

    #[test]
    fn get_children_recurses_two_levels() {
        let mut lvl2 = sw_os("debian", "wheezy");
        let grand = sw_blob("nodejs", "4.8.0");
        lvl2.add_child(grand.clone());

        let mut lvl1 = contract(json!({
            "type": "hw.device-type",
            "slug": "artik10",
            "name": "Artik 10"
        }));
        lvl1.add_child(lvl2.clone());

        let mut root = container();
        root.add_child(lvl1.clone());

        let result = root.get_children();
        assert_eq!(result.len(), 3);
        let hashes: HashSet<&str> = result.iter().map(|c| c.hash()).collect();
        assert!(hashes.contains(lvl1.hash()));
        assert!(hashes.contains(lvl2.hash()));
        assert!(hashes.contains(grand.hash()));
    }

    #[test]
    fn get_children_filter_returns_nested_matches() {
        let mut inner = sw_os("debian", "wheezy");
        let grand = sw_blob("nodejs", "4.8.0");
        inner.add_child(grand.clone());

        let mut parent = container();
        parent.add_child(inner);

        let result = parent.get_children_filtered(&["sw.blob"]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get_type(), "sw.blob");
        assert_eq!(result[0].hash(), grand.hash());
    }

    // get_children_by_type

    #[test]
    fn get_children_by_type_returns_matching_contracts() {
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        let b = sw_os("debian", "jessie");
        let c = sw_os("fedora", "25");
        let d = sw_blob("nodejs", "4.8.0");
        parent.add_children(vec![a.clone(), b.clone(), c.clone(), d]);
        let result = parent.get_children_by_type("sw.os");
        assert_eq!(result.len(), 3);
        assert_eq!(hash_set(&result), hash_set(&[&a, &b, &c]));
    }

    #[test]
    fn get_children_by_type_stable_across_calls() {
        let mut parent = container();
        parent.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            sw_blob("nodejs", "4.8.0"),
        ]);
        let r1 = parent.get_children_by_type("sw.os");
        let r2 = parent.get_children_by_type("sw.os");
        assert_eq!(r1.len(), r2.len());
        assert_eq!(hash_set(&r1), hash_set(&r2));
    }

    #[test]
    fn get_children_by_type_empty_for_unknown_type() {
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy")]);
        assert!(parent.get_children_by_type("arch.sw").is_empty());
    }

    #[test]
    fn get_children_by_type_returns_aliased_contract_once() {
        let mut parent = container();
        let rpi = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi",
            "name": "Raspberry Pi",
            "aliases": ["rpi", "raspberry-pi"]
        }));
        parent.add_children(vec![sw_blob("nodejs", "4.8.0"), rpi.clone()]);
        let result = parent.get_children_by_type("hw.device-type");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], &rpi);
    }

    // get_children_types

    /// Collects the result of [`Contract::get_children_types`] into a
    /// `HashSet` for order-independent comparisons in assertions.
    fn children_type_set(c: &Contract) -> HashSet<String> {
        c.get_children_types().into_iter().collect()
    }

    #[test]
    fn get_children_types_empty_set_when_no_children() {
        let parent = container();
        assert!(parent.get_children_types().is_empty());
    }

    #[test]
    fn get_children_types_single_type_for_one_child() {
        let mut parent = container();
        parent.add_child(sw_os("debian", "wheezy"));
        assert_eq!(children_type_set(&parent), HashSet::from(["sw.os".into()]));
    }

    #[test]
    fn get_children_types_dedupes_same_type() {
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_os("debian", "jessie")]);
        assert_eq!(children_type_set(&parent), HashSet::from(["sw.os".into()]));
    }

    #[test]
    fn get_children_types_union_of_all() {
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_blob("nodejs", "4.8.0")]);
        assert_eq!(
            children_type_set(&parent),
            HashSet::from(["sw.os".into(), "sw.blob".into()])
        );
    }

    #[test]
    fn get_children_types_updates_when_adding() {
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy")]);
        parent.add_children(vec![sw_blob("nodejs", "4.8.0")]);
        assert_eq!(
            children_type_set(&parent),
            HashSet::from(["sw.os".into(), "sw.blob".into()])
        );
    }

    #[test]
    fn get_children_types_recurses_into_nested_children() {
        let mut inner = sw_os("debian", "wheezy");
        inner.add_child(sw_blob("nodejs", "4.8.0"));

        let mut parent = container();
        parent.add_child(inner);

        assert_eq!(
            children_type_set(&parent),
            HashSet::from(["sw.os".into(), "sw.blob".into()])
        );
    }

    #[test]
    fn get_children_types_recurses_two_levels() {
        let mut lvl2 = sw_os("debian", "wheezy");
        lvl2.add_child(sw_blob("nodejs", "4.8.0"));

        let mut lvl1 = contract(json!({
            "type": "hw.device-type",
            "slug": "artik10",
            "name": "Artik 10"
        }));
        lvl1.add_child(lvl2);

        let mut root = container();
        root.add_child(lvl1);

        assert_eq!(
            children_type_set(&root),
            HashSet::from(["hw.device-type".into(), "sw.os".into(), "sw.blob".into()])
        );
    }

    #[test]
    fn get_children_types_returns_deduplicated_vec() {
        // Sanity check: the public API returns a Vec whose length equals
        // its HashSet cardinality (i.e. no duplicates).
        let mut parent = container();
        parent.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            sw_blob("nodejs", "4.8.0"),
        ]);
        let vec = parent.get_children_types();
        let set: HashSet<&str> = vec.iter().map(String::as_str).collect();
        assert_eq!(vec.len(), set.len(), "vec must be deduplicated");
        assert_eq!(vec.len(), 2);
    }

    // Serialization round trip with multiple children

    #[test]
    fn round_trip_with_multi_child_same_slug_tree() {
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_os("debian", "jessie")]);
        let json = serde_json::to_value(&parent).unwrap();
        let reconstructed: Contract = serde_json::from_value(json).unwrap();
        assert_eq!(parent, reconstructed);
    }

    // ── find_children ────────────────────────────────────────────────────

    use crate::types::{ContractType, VersionReq};

    /// Constructs a simple [`ContractMatcher`] from a type / slug /
    /// version triple. `slug` and `version` are left out when
    /// `None`. The returned matcher has no `data` payload — use
    /// [`matcher_with_data`] for tests that need the deep-partial
    /// match path.
    fn matcher(type_: &str, slug: Option<&str>, version: Option<&str>) -> ContractMatcher {
        ContractMatcher::new(
            ContractType::new(type_),
            slug.map(Slug::new),
            version.map(VersionReq::new),
            None,
        )
    }

    /// Constructs a [`ContractMatcher`] for a target type with an
    /// explicit `data` payload. Used by tests that exercise the
    /// deep-partial-match predicate against nested child data.
    fn matcher_with_data(type_: &str, data: Value) -> ContractMatcher {
        ContractMatcher::new(ContractType::new(type_), None, None, Some(data))
    }

    // find_children

    #[test]
    fn find_children_nothing_for_unknown_type() {
        let mut parent = container();
        parent.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            sw_blob("nodejs", "4.8.0"),
        ]);
        let m = matcher("non-existent-type", None, None);
        assert!(parent.find_children(&m).is_empty());
    }

    #[test]
    fn find_children_nothing_for_unknown_type_with_slug() {
        let mut parent = container();
        parent.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            sw_blob("nodejs", "4.8.0"),
        ]);
        let m = matcher("non-existent-type", Some("debian"), None);
        assert!(parent.find_children(&m).is_empty());
    }

    #[test]
    fn find_children_by_type_and_slug() {
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        let b = sw_os("debian", "jessie");
        let c = sw_os("fedora", "25");
        let d = sw_blob("nodejs", "4.8.0");
        parent.add_children(vec![a.clone(), b, c.clone(), d]);

        let m = matcher("sw.os", Some("fedora"), None);
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], &c);
    }

    #[test]
    fn find_children_by_type_and_data_partial_match() {
        // A `data` pattern on the matcher narrows the result set
        // beyond what the `(type, slug)` index provides: only the
        // child whose own `data` is a superset of the matcher
        // pattern is returned.
        let mut parent = container();
        let armv7 = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "data": { "arch": "armv7hf" }
        }));
        let aarch64 = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "jessie",
            "data": { "arch": "aarch64" }
        }));
        parent.add_children(vec![armv7.clone(), aarch64]);

        let m = matcher_with_data("sw.os", json!({ "arch": "armv7hf" }));
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], &armv7);
    }

    #[test]
    fn find_children_multiple_of_one_type() {
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        let b = sw_os("debian", "jessie");
        let c = sw_os("fedora", "25");
        let d = sw_blob("nodejs", "4.8.0");
        parent.add_children(vec![a.clone(), b.clone(), c.clone(), d]);

        let m = matcher("sw.os", None, None);
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 3);
        assert_eq!(hash_set(&result), hash_set(&[&a, &b, &c]));
    }

    #[test]
    fn find_children_by_alias() {
        // The matcher's slug is an alias of the target contract.
        // Because `ChildrenIndex` registers every alias under the
        // same `(type, slug)` entry at insertion time, the index
        // lookup still resolves to the canonical contract.
        let mut parent = container();
        let rpi2 = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi2",
            "name": "Raspberry Pi 2",
            "aliases": ["rpi2", "raspberry-pi2"]
        }));
        let rpi = contract(json!({
            "type": "hw.device-type",
            "slug": "raspberrypi",
            "name": "Raspberry Pi",
            "aliases": ["rpi", "raspberry-pi"]
        }));
        parent.add_children(vec![rpi2, rpi.clone()]);

        let m = matcher("hw.device-type", Some("rpi"), None);
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash(), rpi.hash());
    }

    #[test]
    fn find_children_nested_by_type_and_slug() {
        let mut wheezy = sw_os("debian", "wheezy");
        let nodejs = sw_blob("nodejs", "4.8.0");
        wheezy.add_child(nodejs.clone());

        let mut parent = container();
        parent.add_children(vec![wheezy, sw_os("debian", "jessie")]);

        let m = matcher("sw.blob", Some("nodejs"), None);
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash(), nodejs.hash());
    }

    #[test]
    fn find_children_nested_by_type_and_version_range() {
        // Two `sw.blob` children under different siblings: one
        // above the requested semver range, one below. The range
        // filter must let the 4.8.0 blob through and reject the
        // 3.14.0 blob.
        let mut wheezy = sw_os("debian", "wheezy");
        let nodejs_new = sw_blob("nodejs", "4.8.0");
        wheezy.add_child(nodejs_new.clone());

        let mut jessie = sw_os("debian", "jessie");
        let nodejs_old = sw_blob("nodejs", "3.14.0");
        jessie.add_child(nodejs_old.clone());

        let mut parent = container();
        parent.add_children(vec![wheezy, jessie]);

        let m = matcher("sw.blob", None, Some(">=4.0.0"));
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash(), nodejs_new.hash());

        // And the inverse range pins the other direction: only
        // `nodejs_old` matches `<4.0.0`.
        let m_below = matcher("sw.blob", None, Some("<4.0.0"));
        let result_below = parent.find_children(&m_below);
        assert_eq!(result_below.len(), 1);
        assert_eq!(result_below[0].hash(), nodejs_old.hash());
    }

    #[test]
    fn find_children_nested_fails_on_wrong_slug() {
        let mut wheezy = sw_os("debian", "wheezy");
        let nodejs = sw_blob("nodejs", "4.8.0");
        wheezy.add_child(nodejs);

        let mut parent = container();
        parent.add_children(vec![wheezy, sw_os("debian", "jessie")]);

        let m = matcher("sw.blob", Some("other"), None);
        assert!(parent.find_children(&m).is_empty());
    }

    #[test]
    fn find_children_nested_fails_on_wrong_type() {
        let mut wheezy = sw_os("debian", "wheezy");
        let nodejs = sw_blob("nodejs", "4.8.0");
        wheezy.add_child(nodejs);

        let mut parent = container();
        parent.add_children(vec![wheezy, sw_os("debian", "jessie")]);

        let m = matcher("sw.os", Some("nodejs"), None);
        assert!(parent.find_children(&m).is_empty());
    }

    #[test]
    fn find_children_two_level_nested_by_type() {
        // A leaf `sw.blob` lives two levels down from `root`.
        // The matcher only carries the target type, so the
        // `has_only_type` fast path runs and every child under
        // `by_type["sw.blob"]` at any depth of the walk is
        // emitted.
        let mut lvl2 = sw_os("debian", "wheezy");
        let leaf = sw_blob("nodejs", "4.8.0");
        lvl2.add_child(leaf.clone());
        let mut lvl1 = contract(json!({
            "type": "hw.device-type",
            "slug": "artik10",
            "name": "Artik 10"
        }));
        lvl1.add_child(lvl2);

        let mut root = container();
        root.add_child(lvl1);

        let m = matcher("sw.blob", None, None);
        let result = root.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash(), leaf.hash());
    }

    #[test]
    fn find_children_nested_data_matcher() {
        // Arbitrary matching criteria live inside the matcher's
        // `data` payload, so the child must carry the same nesting
        // under its own `data` for the deep partial match to hit.
        let mut parent = container();
        let armv7 = contract(json!({
            "type": "hw.device-type",
            "slug": "artik10",
            "name": "Samsung Artik 10",
            "data": { "arch": "armv7hf" }
        }));
        let aarch64 = contract(json!({
            "type": "hw.device-type",
            "slug": "pine64",
            "name": "Pine A64",
            "data": { "arch": "aarch64" }
        }));
        parent.add_children(vec![armv7.clone(), aarch64]);

        let m = matcher_with_data("hw.device-type", json!({ "arch": "armv7hf" }));
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash(), armv7.hash());
    }

    #[test]
    fn find_children_data_pattern_rejects_child_without_data() {
        // Regression guard for the `None`-guard on `child.body.data`:
        // a matcher carrying a non-trivial `data` pattern must reject
        // any child that has no `data` field at all, rather than
        // matching it against an implicit `Value::Null`.
        let mut parent = container();
        let with_data = contract(json!({
            "type": "hw.device-type",
            "slug": "artik10",
            "data": { "arch": "armv7hf" }
        }));
        let without_data = contract(json!({
            "type": "hw.device-type",
            "slug": "nodata"
        }));
        parent.add_children(vec![with_data.clone(), without_data]);

        let m = matcher_with_data("hw.device-type", json!({ "arch": "armv7hf" }));
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash(), with_data.hash());
    }

    // find_children caching

    #[test]
    fn find_children_is_stable_across_calls() {
        // Running the same matcher twice on the same parent must
        // return the same result. The second call goes through the
        // cache path, so this pins the cache-hit resolution against
        // the cache-miss path.
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        let b = sw_os("fedora", "25");
        parent.add_children(vec![a.clone(), b]);

        let m = matcher("sw.os", Some("debian"), None);

        let first = {
            let refs = parent.find_children(&m);
            refs.iter()
                .map(|c| c.hash().to_string())
                .collect::<Vec<_>>()
        };
        let second = {
            let refs = parent.find_children(&m);
            refs.iter()
                .map(|c| c.hash().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(first.len(), 1);
        assert_eq!(first, second);
        assert_eq!(first[0], a.hash());
    }

    #[test]
    fn find_children_empty_result_is_still_stable() {
        // Searching for something that does not exist must still
        // return an empty Vec on every call.
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_os("fedora", "25")]);
        let m = matcher("sw.os", Some("alpine"), None);
        assert!(parent.find_children(&m).is_empty());
        assert!(parent.find_children(&m).is_empty());
    }

    #[test]
    fn find_children_cache_is_invalidated_after_add() {
        // Populate the cache, then add another matching child, then
        // search again. The new child must appear in the second
        // result — if it didn't, the stale cache entry would mask it.
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        parent.add_child(a.clone());

        let m = matcher("sw.os", None, None);
        let first: HashSet<String> = parent
            .find_children(&m)
            .iter()
            .map(|c| c.hash().to_string())
            .collect();
        assert_eq!(first.len(), 1);

        let b = sw_os("debian", "jessie");
        parent.add_child(b.clone());

        let second: HashSet<String> = parent
            .find_children(&m)
            .iter()
            .map(|c| c.hash().to_string())
            .collect();
        assert_eq!(second.len(), 2);
        assert!(second.contains(a.hash()));
        assert!(second.contains(b.hash()));
    }

    #[test]
    fn find_children_cache_is_invalidated_after_remove() {
        // Same as above but for removal. Removing a hit must cause
        // it to disappear from the next search result.
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        let b = sw_os("debian", "jessie");
        parent.add_children(vec![a.clone(), b.clone()]);

        let m = matcher("sw.os", None, None);
        let first: HashSet<String> = parent
            .find_children(&m)
            .iter()
            .map(|c| c.hash().to_string())
            .collect();
        assert_eq!(first.len(), 2);

        parent.remove_child(&b);

        let second: HashSet<String> = parent
            .find_children(&m)
            .iter()
            .map(|c| c.hash().to_string())
            .collect();
        assert_eq!(second.len(), 1);
        assert!(second.contains(a.hash()));
        assert!(!second.contains(b.hash()));
    }

    #[test]
    fn find_children_cache_survives_failed_remove() {
        // Removing a contract that isn't present must leave the
        // cache intact. `ChildrenIndex::remove` returns early on
        // missing hashes without touching the search cache, so the
        // prior entry is still valid on the next search.
        let mut parent = container();
        let a = sw_os("debian", "wheezy");
        parent.add_child(a.clone());

        let m = matcher("sw.os", None, None);
        let _ = parent.find_children(&m);

        // `other` was never added to parent, so `remove_child` is
        // a full no-op.
        let other = sw_os("debian", "sid");
        parent.remove_child(&other);

        let second: HashSet<String> = parent
            .find_children(&m)
            .iter()
            .map(|c| c.hash().to_string())
            .collect();
        assert_eq!(second.len(), 1);
        assert!(second.contains(a.hash()));
    }

    #[test]
    fn find_children_cache_scoped_by_type() {
        // Two matchers targeting different types must each yield
        // their own independent cache entry. Add a child of one
        // type; the other type's cache entry must remain valid.
        let mut parent = container();
        let os = sw_os("debian", "wheezy");
        let blob = sw_blob("nodejs", "4.8.0");
        parent.add_children(vec![os.clone(), blob.clone()]);

        let m_os = matcher("sw.os", None, None);
        let m_blob = matcher("sw.blob", None, None);

        let _ = parent.find_children(&m_os);
        let _ = parent.find_children(&m_blob);

        // Add another sw.os — the sw.blob cache entry must still be
        // usable and return the single existing blob.
        parent.add_child(sw_os("debian", "jessie"));
        let blob_result = parent.find_children(&m_blob);
        assert_eq!(blob_result.len(), 1);
        assert_eq!(blob_result[0].hash(), blob.hash());
    }

    #[test]
    fn find_children_cache_invalidated_on_subtree_insert_of_cached_type() {
        // Regression guard: adding a new child whose own type is
        // unrelated to a previously cached target type but whose
        // subtree carries grandchildren of that cached type must
        // invalidate the cache. Without invalidation keyed on the
        // inbound subtree's type closure, the second search would
        // silently return the stale single-blob result.
        let mut root = container();
        let blob1 = sw_blob("nodejs", "4.8.0");
        root.add_children(vec![sw_os("debian", "wheezy"), blob1.clone()]);

        let m = matcher("sw.blob", None, None);
        let first: HashSet<String> = root
            .find_children(&m)
            .iter()
            .map(|c| c.hash().to_string())
            .collect();
        assert_eq!(first.len(), 1);
        assert!(first.contains(blob1.hash()));

        // A new sw.os sibling that carries its own sw.blob grandchild.
        let mut jessie = sw_os("debian", "jessie");
        let blob2 = sw_blob("nodejs", "5.0.0");
        jessie.add_child(blob2.clone());
        root.add_child(jessie);

        let second: HashSet<String> = root
            .find_children(&m)
            .iter()
            .map(|c| c.hash().to_string())
            .collect();
        assert_eq!(second.len(), 2);
        assert!(second.contains(blob1.hash()));
        assert!(second.contains(blob2.hash()));
    }

    #[test]
    fn find_children_known_type_unknown_slug_returns_empty() {
        // Type exists in the index but the slug does not. The
        // `(type, slug)` lookup yields an empty iterator and the
        // search walks no candidates. The result must be empty
        // on both the cold cache miss and any subsequent call.
        let mut parent = container();
        parent.add_children(vec![sw_os("debian", "wheezy"), sw_os("debian", "jessie")]);

        let m = matcher("sw.os", Some("alpine"), None);
        assert!(parent.find_children(&m).is_empty());
        assert!(parent.find_children(&m).is_empty());
    }

    // provides → children conversion

    #[test]
    fn provides_becomes_child_found_by_type() {
        let mut parent = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {
                    "type": "sw.os",
                    "slug": "debian",
                    "version": "wheezy"
                }
            ]
        }));
        parent.add_child(ctx);

        let m = matcher("sw.os", None, None);
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw.kind.as_str(), "sw.os");
    }

    #[test]
    fn provides_wrong_type_not_found() {
        let mut parent = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {
                    "type": "sw.os",
                    "slug": "debian"
                }
            ]
        }));
        parent.add_child(ctx);

        let m = matcher("sw.blob", None, None);
        assert!(parent.find_children(&m).is_empty());
    }

    #[test]
    fn provides_found_by_type_and_slug() {
        let mut parent = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {
                    "type": "sw.os",
                    "slug": "debian",
                    "version": "wheezy"
                }
            ]
        }));
        parent.add_child(ctx);

        let m = matcher("sw.os", Some("debian"), None);
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw.kind.as_str(), "sw.os");
        assert_eq!(result[0].raw.body.slug.as_ref().unwrap().as_str(), "debian");
    }

    #[test]
    fn provides_found_by_version_range() {
        let mut parent = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {
                    "type": "sw.blob",
                    "slug": "nodejs",
                    "version": "4.8.0"
                }
            ]
        }));
        parent.add_child(ctx);

        let m = matcher("sw.blob", None, Some(">=4.0.0"));
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw.kind.as_str(), "sw.blob");
    }

    #[test]
    fn provides_version_range_miss() {
        let mut parent = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {
                    "type": "sw.blob",
                    "slug": "nodejs",
                    "version": "3.0.0"
                }
            ]
        }));
        parent.add_child(ctx);

        let m = matcher("sw.blob", None, Some(">=4.0.0"));
        assert!(parent.find_children(&m).is_empty());
    }

    #[test]
    fn provides_multiple_same_type_become_separate_children() {
        let mut parent = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {
                    "type": "sw.blob",
                    "slug": "nodejs",
                    "version": "4.8.0"
                },
                {
                    "type": "sw.blob",
                    "slug": "nodejs",
                    "version": "5.0.0"
                }
            ]
        }));
        parent.add_child(ctx);

        let m = matcher("sw.blob", None, None);
        let result = parent.find_children(&m);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn provides_on_root_become_children() {
        let mut root = contract(json!({
            "type": "meta.context",
            "slug": "root",
            "provides": [
                {
                    "type": "sw.os",
                    "slug": "debian"
                }
            ]
        }));
        let m = matcher("sw.os", Some("debian"), None);
        let result = root.find_children(&m);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw.kind.as_str(), "sw.os");
    }

    // ── requirement satisfaction ─────────────────────────────────────────

    /// Minimal `hw.device-type` contract helper. Any matching
    /// criteria beyond type and slug must live inside `data` per the
    /// CUE schema; this helper intentionally only carries the
    /// identifier fields so tests that exercise deep-partial matching
    /// can layer their own `data` payload on top via the JSON
    /// literal.
    fn hw_device(slug: &str) -> Contract {
        contract(json!({
            "type": "hw.device-type",
            "slug": slug,
            "name": slug,
        }))
    }

    /// Builds a `sw.stack` contract with a requirement list. Used by
    /// the satisfaction tests to construct child contracts whose
    /// `requires` shape drives the conjunct walk.
    fn stack_with_requires(requires: Value) -> Contract {
        contract(json!({
            "type": "sw.stack",
            "slug": "nodejs",
            "name": "Node.js",
            "requires": requires,
        }))
    }

    // satisfies_child_contract

    #[test]
    fn satisfies_child_contract_empty_self_and_no_requirements_returns_true() {
        let mut container = container();
        let child = contract(json!({
            "type": "test",
            "slug": "foo",
            "name": "Foo",
            "version": "1.2.3"
        }));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_empty_self_with_requirements_returns_false() {
        let mut container = container();
        let child = stack_with_requires(json!([
            {"type": "sw.arch", "slug": "amd64"}
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_with_no_child_requirements_returns_true() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy"), sw_os("debian", "jessie")]);
        let child = contract(json!({
            "type": "test",
            "slug": "foo",
            "name": "Foo",
            "version": "1.2.3"
        }));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_one_fulfilled_requirement_returns_true() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy"), sw_os("debian", "jessie")]);
        let child = stack_with_requires(json!([
            {"slug": "debian", "version": "wheezy", "type": "sw.os"}
        ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_two_fulfilled_requirements_returns_true() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"slug": "debian", "version": "wheezy", "type": "sw.os"},
            {"slug": "artik10", "type": "hw.device-type"}
        ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_one_unfulfilled_requirement_returns_false() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"slug": "void", "type": "sw.os"}
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_one_of_two_unfulfilled_returns_false() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"slug": "void", "type": "sw.os"},
            {"slug": "artik10", "type": "hw.device-type"}
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_empty_disjunction_is_satisfied() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([ { "or": [] } ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_partial_not_operator_returns_false() {
        // `not` is violated if ANY negated disjunct has a match, so a
        // partial hit (fedora missing, debian present) still fails.
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy")]);
        let child = stack_with_requires(json!([
            {
                "not": [
                    {"slug": "fedora", "type": "sw.os"},
                    {"slug": "debian", "type": "sw.os"}
                ]
            }
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_unfulfilled_not_operator_returns_false() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy")]);
        let child = stack_with_requires(json!([
            {"not": [{"slug": "debian", "type": "sw.os"}]}
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_fulfilled_not_operator_returns_true() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy")]);
        let child = stack_with_requires(json!([
            {"not": [{"slug": "foo-bar", "type": "sw.os"}]}
        ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_empty_not_operator_is_satisfied() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy")]);
        let child = stack_with_requires(json!([ {"not": []} ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_two_unfulfilled_returns_false() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"slug": "void", "type": "sw.os"},
            {"slug": "raspberry-pi", "type": "hw.device-type"}
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_one_fulfilled_disjunction_returns_true() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"or": [{"slug": "debian", "type": "sw.os"}]}
        ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_mixed_disjunction_returns_true() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {
                "or": [
                    {"slug": "debian", "type": "sw.os"},
                    {"slug": "void", "type": "sw.os"}
                ]
            }
        ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_one_unfulfilled_disjunction_returns_false() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"or": [{"slug": "void", "type": "sw.os"}]}
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_empty_disjunction_and_unfulfilled_returns_false() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"or": []},
            {"slug": "void", "type": "sw.os"}
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_fulfilled_disjunction_and_unfulfilled_returns_false() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {
                "or": [
                    {"type": "sw.os", "slug": "void"},
                    {"type": "sw.os", "slug": "debian"}
                ]
            },
            {"slug": "raspberry-pi", "type": "hw.device-type"}
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_types_filter_limits_evaluation() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        // Requires `test` type that doesn't exist; only `sw.os` is
        // evaluated so the `test` requirement is skipped.
        let child = stack_with_requires(json!([
            {"slug": "debian", "type": "sw.os"},
            {"slug": "hello", "type": "test"}
        ]));
        let allowed: &[&str] = &["sw.os"];
        assert!(container.satisfies_child_contract(&child, Some(allowed)));
    }

    #[test]
    fn satisfies_child_contract_types_filter_allows_multiple_types() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"slug": "debian", "type": "sw.os"},
            {
                "or": [
                    {"type": "hw.device-type", "slug": "artik10"},
                    {"type": "hw.device-type", "slug": "raspberry-pi"}
                ]
            },
            {"slug": "hello", "type": "test"}
        ]));
        let allowed: &[&str] = &["sw.os", "hw.device-type"];
        assert!(container.satisfies_child_contract(&child, Some(allowed)));
    }

    #[test]
    fn satisfies_child_contract_types_filter_with_unfulfilled_requirement_returns_false() {
        let mut container = container();
        container.add_children(vec![
            sw_os("debian", "wheezy"),
            sw_os("debian", "jessie"),
            hw_device("artik10"),
        ]);
        let child = stack_with_requires(json!([
            {"slug": "void", "type": "sw.os"}
        ]));
        let allowed: &[&str] = &["sw.os"];
        assert!(!container.satisfies_child_contract(&child, Some(allowed)));
    }

    #[test]
    fn satisfies_child_contract_unfulfilled_disjunction_of_non_selected_type_returns_true() {
        // Every disjunct targets a type not in the allowed set, so
        // the filtered disjunction is empty and trivially satisfied.
        let mut container = container();
        container.add_children(vec![hw_device("artik10")]);
        let child = contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "name": "Debian",
            "requires": [
                {
                    "or": [
                        {"type": "hw.device-type", "slug": "intel-edison"},
                        {"type": "hw.device-type", "slug": "raspberry-pi"}
                    ]
                }
            ]
        }));
        let allowed: &[&str] = &["arch.sw"];
        assert!(container.satisfies_child_contract(&child, Some(allowed)));
    }

    #[test]
    fn satisfies_child_contract_composite_context_two_requirements() {
        // A composite child contract brings its own children into the
        // conjunct set via the recursive `getChildren` walk.
        let mut container = container();

        let composite = contract(json!({
            "type": "meta.composite",
            "slug": "test",
            "children": {
                "sw.os": {"type": "sw.os", "slug": "debian", "version": "wheezy"},
                "arch.sw": {"type": "arch.sw", "slug": "amd64", "version": "1"}
            }
        }));
        container.add_child(composite);

        let child = stack_with_requires(json!([
            {"slug": "debian", "type": "sw.os"},
            {
                "or": [
                    {"slug": "amd64", "type": "arch.sw"},
                    {"slug": "i386", "type": "arch.sw"}
                ]
            }
        ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_capabilities_via_provides() {
        // A context contract with `provides` entries that become children
        // must satisfy the child's requirements via `find_children`.
        let mut container = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {"type": "sw.os", "slug": "debian", "version": "wheezy"},
                {"type": "arch.sw", "slug": "amd64", "version": "1"}
            ]
        }));
        container.add_child(ctx);

        let child = stack_with_requires(json!([
            {"slug": "debian", "type": "sw.os"},
            {
                "or": [
                    {"slug": "amd64", "type": "arch.sw"},
                    {"slug": "i386", "type": "arch.sw"}
                ]
            }
        ]));
        assert!(container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_capabilities_with_types_filter_returns_true() {
        // The context only provides `sw.os`; the `arch.sw`
        // disjunction is ignored because only `sw.os` is in scope.
        let mut container = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {"type": "sw.os", "slug": "debian", "version": "wheezy"}
            ]
        }));
        container.add_child(ctx);

        let child = stack_with_requires(json!([
            {"slug": "debian", "type": "sw.os"},
            {
                "or": [
                    {"slug": "amd64", "type": "arch.sw"},
                    {"slug": "i386", "type": "arch.sw"}
                ]
            }
        ]));
        let allowed: &[&str] = &["sw.os"];
        assert!(container.satisfies_child_contract(&child, Some(allowed)));
    }

    #[test]
    fn satisfies_child_contract_capabilities_missing_disjunct_returns_false() {
        // The context only provides `sw.os`; the disjunction over
        // `arch.sw` has no applicable matches so the whole thing
        // fails. `get_not_satisfied_child_requirements` reports
        // exactly one unsatisfied conjunct.
        let mut container = container();
        let ctx = contract(json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {"type": "sw.os", "slug": "debian", "version": "wheezy"}
            ]
        }));
        container.add_child(ctx);

        let child = stack_with_requires(json!([
            {"slug": "debian", "type": "sw.os"},
            {
                "or": [
                    {"slug": "amd64", "type": "arch.sw"},
                    {"slug": "i386", "type": "arch.sw"}
                ]
            }
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
        assert_eq!(
            container
                .get_not_satisfied_child_requirements(&child, None)
                .len(),
            1
        );
    }

    #[test]
    fn satisfies_child_contract_composite_one_unfulfilled_returns_false() {
        let mut container = container();
        let composite = contract(json!({
            "type": "meta.composite",
            "slug": "test",
            "children": {
                "sw.os": {"type": "sw.os", "slug": "debian", "version": "wheezy"},
                "arch.sw": {"type": "arch.sw", "slug": "amd64", "version": "1"}
            }
        }));
        container.add_child(composite);

        let child = stack_with_requires(json!([
            {"slug": "fedora", "type": "sw.os"},
            {
                "or": [
                    {"slug": "amd64", "type": "arch.sw"},
                    {"slug": "i386", "type": "arch.sw"}
                ]
            }
        ]));
        assert!(!container.satisfies_child_contract(&child, None));
    }

    #[test]
    fn satisfies_child_contract_composite_unfulfilled_ignored_by_types_filter() {
        // The unfulfilled `sw.os` requirement is outside the type
        // filter, so only the fulfilled `arch.sw` disjunction counts.
        let mut container = container();
        let composite = contract(json!({
            "type": "meta.composite",
            "slug": "test",
            "children": {
                "sw.os": {"type": "sw.os", "slug": "debian", "version": "wheezy"},
                "arch.sw": {"type": "arch.sw", "slug": "amd64", "version": "1"}
            }
        }));
        container.add_child(composite);

        let child = stack_with_requires(json!([
            {"slug": "fedora", "type": "sw.os"},
            {
                "or": [
                    {"slug": "amd64", "type": "arch.sw"},
                    {"slug": "i386", "type": "arch.sw"}
                ]
            }
        ]));
        let allowed: &[&str] = &["arch.sw"];
        assert!(container.satisfies_child_contract(&child, Some(allowed)));
    }

    #[test]
    fn satisfies_child_contract_fulfilled_composite_argument() {
        // `satisfies_child_contract` walks the argument's descendants
        // for compiled requirements. A composite child whose
        // grandchildren require `arch.sw amd64` is satisfied when
        // the container has `arch.sw amd64` as a direct child.
        let mut container = container();
        let arch = contract(json!({
            "type": "arch.sw",
            "slug": "amd64"
        }));
        container.add_child(arch);

        let composite = contract(json!({
            "type": "meta.composite",
            "slug": "test",
            "children": {
                "sw.os": {
                    "type": "sw.os",
                    "slug": "debian",
                    "version": "wheezy",
                    "requires": [{"type": "arch.sw", "slug": "amd64"}]
                }
            }
        }));
        assert!(container.satisfies_child_contract(&composite, None));
    }

    #[test]
    fn satisfies_child_contract_unfulfilled_composite_argument() {
        let mut container = container();
        let arch = contract(json!({
            "type": "arch.sw",
            "slug": "amd64"
        }));
        container.add_child(arch);

        let composite = contract(json!({
            "type": "meta.composite",
            "slug": "test",
            "children": {
                "sw.os": {
                    "type": "sw.os",
                    "slug": "debian",
                    "version": "wheezy",
                    "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
                }
            }
        }));
        assert!(!container.satisfies_child_contract(&composite, None));
    }

    // are_children_satisfied

    #[test]
    fn are_children_satisfied_returns_true_for_satisfied_context() {
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "name": "Debian",
                "slug": "debian",
                "requires": [
                    {
                        "or": [
                            {"type": "hw.device-type", "slug": "artik10"},
                            {"type": "hw.device-type", "slug": "raspberry-pi"}
                        ]
                    }
                ]
            })),
            contract(json!({
                "type": "hw.device-type",
                "slug": "artik10",
                "name": "Samsung Artik 10",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf",
                "name": "armv7hf"
            })),
        ]);
        assert!(container.are_children_satisfied(None));
    }

    #[test]
    fn are_children_satisfied_returns_false_for_unsatisfied_context() {
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "name": "Debian",
                "slug": "debian",
                "requires": [
                    {
                        "or": [
                            {"type": "hw.device-type", "slug": "artik10"},
                            {"type": "hw.device-type", "slug": "raspberry-pi"}
                        ]
                    }
                ]
            })),
            contract(json!({
                "type": "hw.device-type",
                "slug": "artik10",
                "name": "Samsung Artik 10",
                "requires": [{"type": "arch.sw", "slug": "amd64"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf",
                "name": "armv7hf"
            })),
        ]);
        assert!(!container.are_children_satisfied(None));
    }

    #[test]
    fn are_children_satisfied_returns_false_for_missing_requirement() {
        let mut container = container();
        container.add_children(vec![contract(json!({
            "type": "sw.os",
            "name": "Debian",
            "slug": "debian",
            "requires": [{"type": "hw.device-type", "slug": "artik10"}]
        }))]);
        assert!(!container.are_children_satisfied(None));
    }

    #[test]
    fn are_children_satisfied_skips_disjoint_types() {
        // The child requires `hw.device-type` but the filter only
        // cares about `arch.sw` — the child's requirement types are
        // disjoint with the filter, so the child is skipped and the
        // overall check is trivially satisfied.
        let mut container = container();
        container.add_children(vec![contract(json!({
            "type": "sw.os",
            "name": "Debian",
            "slug": "debian",
            "requires": [{"type": "hw.device-type", "slug": "artik10"}]
        }))]);
        let allowed: &[&str] = &["arch.sw"];
        assert!(container.are_children_satisfied(Some(allowed)));
    }

    #[test]
    fn are_children_satisfied_types_filter_satisfied_subset() {
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "name": "Debian",
                "slug": "debian",
                "requires": [
                    {
                        "or": [
                            {"type": "hw.device-type", "slug": "artik10"},
                            {"type": "hw.device-type", "slug": "raspberry-pi"}
                        ]
                    }
                ]
            })),
            contract(json!({
                "type": "hw.device-type",
                "slug": "artik10",
                "name": "Samsung Artik 10",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf",
                "name": "armv7hf"
            })),
        ]);
        let allowed: &[&str] = &["hw.device-type"];
        assert!(container.are_children_satisfied(Some(allowed)));
    }

    #[test]
    fn are_children_satisfied_types_filter_satisfied_in_unsatisfied_context() {
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "name": "Debian",
                "slug": "debian",
                "requires": [
                    {
                        "or": [
                            {"type": "hw.device-type", "slug": "intel-edison"},
                            {"type": "hw.device-type", "slug": "raspberry-pi"}
                        ]
                    }
                ]
            })),
            contract(json!({
                "type": "hw.device-type",
                "slug": "artik10",
                "name": "Samsung Artik 10",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf",
                "name": "armv7hf"
            })),
        ]);
        let allowed: &[&str] = &["arch.sw"];
        assert!(container.are_children_satisfied(Some(allowed)));
    }

    #[test]
    fn are_children_satisfied_unknown_type_filter_in_unsatisfied_context() {
        // No child references `foo` in its requirements, so the
        // filter is disjoint for every child and the whole check is
        // trivially satisfied.
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "name": "Debian",
                "slug": "debian",
                "requires": [
                    {
                        "or": [
                            {"type": "hw.device-type", "slug": "intel-edison"},
                            {"type": "hw.device-type", "slug": "raspberry-pi"}
                        ]
                    }
                ]
            })),
            contract(json!({
                "type": "hw.device-type",
                "slug": "artik10",
                "name": "Samsung Artik 10",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf",
                "name": "armv7hf"
            })),
        ]);
        let allowed: &[&str] = &["foo"];
        assert!(container.are_children_satisfied(Some(allowed)));
    }

    #[test]
    fn are_children_satisfied_types_filter_unsatisfied_returns_false() {
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "name": "Debian",
                "slug": "debian",
                "requires": [
                    {
                        "or": [
                            {"type": "hw.device-type", "slug": "intel-edison"},
                            {"type": "hw.device-type", "slug": "raspberry-pi"}
                        ]
                    }
                ]
            })),
            contract(json!({
                "type": "hw.device-type",
                "slug": "artik10",
                "name": "Samsung Artik 10",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf",
                "name": "armv7hf"
            })),
        ]);
        let allowed: &[&str] = &["hw.device-type"];
        assert!(!container.are_children_satisfied(Some(allowed)));
    }

    #[test]
    fn are_children_satisfied_mixed_filter_returns_false_when_one_type_unsatisfied() {
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "name": "Debian",
                "slug": "debian",
                "requires": [
                    {
                        "or": [
                            {"type": "hw.device-type", "slug": "intel-edison"},
                            {"type": "hw.device-type", "slug": "raspberry-pi"}
                        ]
                    }
                ]
            })),
            contract(json!({
                "type": "hw.device-type",
                "slug": "artik10",
                "name": "Samsung Artik 10",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf",
                "name": "armv7hf"
            })),
        ]);
        let allowed: &[&str] = &["arch.sw", "hw.device-type"];
        assert!(!container.are_children_satisfied(Some(allowed)));
    }

    #[test]
    fn are_children_satisfied_two_satisfied_types_returns_true() {
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "name": "Debian",
                "slug": "debian",
                "requires": [
                    {
                        "or": [
                            {"type": "hw.device-type", "slug": "artik10"},
                            {"type": "hw.device-type", "slug": "raspberry-pi"}
                        ]
                    }
                ]
            })),
            contract(json!({
                "type": "hw.device-type",
                "slug": "artik10",
                "name": "Samsung Artik 10",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf",
                "name": "armv7hf"
            })),
        ]);
        let allowed: &[&str] = &["arch.sw", "hw.device-type"];
        assert!(container.are_children_satisfied(Some(allowed)));
    }

    #[test]
    fn are_children_satisfied_nested_contexts_all_satisfied() {
        // A container with three `child` contracts, each wrapping a
        // sub-contract — the recursive `get_children` walk must
        // discover the nested requirements across sibling subtrees.
        let container = contract(json!({
            "type": "foo",
            "slug": "bar",
            "children": {
                "child": {
                    "child-1": {
                        "type": "child",
                        "slug": "child-1",
                        "children": {
                            "sw.os": {
                                "type": "sw.os",
                                "name": "Debian",
                                "slug": "debian",
                                "requires": [
                                    {"type": "hw.device-type", "slug": "artik10"}
                                ]
                            }
                        }
                    },
                    "child-2": {
                        "type": "child",
                        "slug": "child-2",
                        "children": {
                            "hw.device-type": {
                                "type": "hw.device-type",
                                "slug": "artik10",
                                "name": "Samsung Artik 10",
                                "requires": [
                                    {"type": "arch.sw", "slug": "armv7hf"}
                                ]
                            }
                        }
                    },
                    "child-3": {
                        "type": "child",
                        "slug": "child-3",
                        "children": {
                            "arch.sw": {
                                "type": "arch.sw",
                                "slug": "armv7hf",
                                "name": "armv7hf"
                            }
                        }
                    }
                }
            }
        }));
        let mut container = container;
        assert!(container.are_children_satisfied(None));
    }

    #[test]
    fn are_children_satisfied_nested_contexts_unsatisfied() {
        let mut container = contract(json!({
            "type": "foo",
            "slug": "bar",
            "children": {
                "child": {
                    "child-1": {
                        "type": "child",
                        "slug": "child-1",
                        "children": {
                            "sw.os": {
                                "type": "sw.os",
                                "name": "Debian",
                                "slug": "debian",
                                "requires": [
                                    {"type": "hw.device-type", "slug": "artik10"}
                                ]
                            }
                        }
                    },
                    "child-2": {
                        "type": "child",
                        "slug": "child-2",
                        "children": {
                            "hw.device-type": {
                                "type": "hw.device-type",
                                "slug": "artik10",
                                "name": "Samsung Artik 10",
                                "requires": [
                                    {"type": "arch.sw", "slug": "armv7hf"}
                                ]
                            }
                        }
                    },
                    "child-3": {
                        "type": "child",
                        "slug": "child-3",
                        "children": {
                            "arch.sw": {
                                "type": "arch.sw",
                                "slug": "armel",
                                "name": "armel"
                            }
                        }
                    }
                }
            }
        }));
        assert!(!container.are_children_satisfied(None));
    }

    // get_not_satisfied_child_requirements

    #[test]
    fn get_not_satisfied_empty_when_no_requirements() {
        let mut container = container();
        let child = contract(json!({
            "type": "test",
            "slug": "foo",
            "version": "1.0.0"
        }));
        let out = container.get_not_satisfied_child_requirements(&child, None);
        assert!(out.is_empty());
    }

    #[test]
    fn get_not_satisfied_reports_single_unfulfilled_match() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy")]);
        let child = stack_with_requires(json!([
            {"slug": "void", "type": "sw.os"}
        ]));
        let out = container.get_not_satisfied_child_requirements(&child, None);
        assert_eq!(out.len(), 1);
        match &out[0] {
            ContractRequirement::Match(m) => {
                assert_eq!(m.kind.as_str(), "sw.os");
                assert_eq!(m.slug.as_ref().unwrap().as_str(), "void");
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn get_not_satisfied_reports_all_unfulfilled_conjuncts() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy")]);
        let child = stack_with_requires(json!([
            {"slug": "void", "type": "sw.os"},
            {"slug": "missing", "type": "hw.device-type"}
        ]));
        let out = container.get_not_satisfied_child_requirements(&child, None);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn get_not_satisfied_empty_for_fulfilled_child() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy")]);
        let child = stack_with_requires(json!([
            {"slug": "debian", "type": "sw.os", "version": "wheezy"}
        ]));
        let out = container.get_not_satisfied_child_requirements(&child, None);
        assert!(out.is_empty());
    }

    // get_all_not_satisfied_child_requirements

    #[test]
    fn get_all_not_satisfied_empty_when_everything_satisfied() {
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "slug": "debian",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "arch.sw",
                "slug": "armv7hf"
            })),
        ]);
        let out = container.get_all_not_satisfied_child_requirements(None);
        assert!(out.is_empty());
    }

    #[test]
    fn get_all_not_satisfied_reports_missing_requirements() {
        let mut container = container();
        container.add_children(vec![contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
        }))]);
        let out = container.get_all_not_satisfied_child_requirements(None);
        assert_eq!(out.len(), 1);
        match &out[0] {
            ContractRequirement::Match(m) => {
                assert_eq!(m.kind.as_str(), "arch.sw");
                assert_eq!(m.slug.as_ref().unwrap().as_str(), "armv7hf");
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn get_all_not_satisfied_disjoint_filter_reports_own_compiled_requirements() {
        // The child's own compiled requirements target `arch.sw`,
        // but the filter only allows `hw.device-type`. The disjoint
        // branch reports the child's own compiled requirements
        // wholesale because they can't possibly be satisfied within
        // the current scope.
        let mut container = container();
        container.add_children(vec![contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
        }))]);
        let allowed: &[&str] = &["hw.device-type"];
        let out = container.get_all_not_satisfied_child_requirements(Some(allowed));
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn get_all_not_satisfied_empty_when_child_has_no_requirements() {
        let mut container = container();
        container.add_children(vec![sw_os("debian", "wheezy")]);
        let out = container.get_all_not_satisfied_child_requirements(None);
        assert!(out.is_empty());
    }

    #[test]
    fn get_all_not_satisfied_mixed_disjoint_and_normal_children() {
        // Two descendants with different requirement types. The filter
        // matches one (`arch.sw`) and misses the other
        // (`hw.device-type` is disjoint from the `arch.sw` filter).
        // Both branches of the per-descendant loop must execute in one
        // call and their outputs concatenated into a single vector:
        //
        // - disjoint child → reports its own compiled requirement
        //   wholesale (the `hw.device-type` match)
        // - non-disjoint child → evaluated normally; reports its
        //   single unsatisfied `arch.sw` match because no child of
        //   that type exists in the container
        let mut container = container();
        container.add_children(vec![
            contract(json!({
                "type": "sw.os",
                "slug": "debian",
                "requires": [{"type": "arch.sw", "slug": "armv7hf"}]
            })),
            contract(json!({
                "type": "sw.stack",
                "slug": "nodejs",
                "requires": [{"type": "hw.device-type", "slug": "artik10"}]
            })),
        ]);
        let allowed: &[&str] = &["arch.sw"];
        let out = container.get_all_not_satisfied_child_requirements(Some(allowed));
        assert_eq!(out.len(), 2);
        let kinds: HashSet<&str> = out
            .iter()
            .map(|r| match r {
                ContractRequirement::Match(m) => m.kind.as_str(),
                other => panic!("expected Match, got {other:?}"),
            })
            .collect();
        assert!(kinds.contains("arch.sw"));
        assert!(kinds.contains("hw.device-type"));
    }

    #[test]
    fn are_children_satisfied_with_none_filter_does_not_disjoint_skip() {
        // Pin the invariant that passing `None` for `types`
        // evaluates every descendant's requirements regardless of
        // the descendant's requirement-types set. The single
        // descendant references an unknown type; without a filter,
        // the requirement must be checked and found unsatisfied.
        let mut container = container();
        container.add_children(vec![contract(json!({
            "type": "sw.os",
            "slug": "debian",
            "requires": [{"type": "hw.device-type", "slug": "artik10"}]
        }))]);
        assert!(!container.are_children_satisfied(None));
    }

    #[test]
    fn are_children_satisfied_cross_subtree_with_types_filter() {
        // Pins the invariant that `root_children` in
        // `check_descendants_satisfied_recursive` is the full
        // recursive search scope — so a deeply nested descendant in
        // one subtree can satisfy another subtree's requirement
        // even with a non-trivial `types` filter applied.
        //
        // Tree:
        //   container
        //   ├─ group-1 (type=group)
        //   │    └─ sw.os debian requires {hw.device-type artik10}
        //   └─ group-2 (type=group)
        //        └─ sub (type=subgroup)
        //             └─ hw.device-type artik10  (satisfies group-1's grandchild)
        //
        // With types=["hw.device-type"]:
        // - `group-1` has no direct requirement types → disjoint → skip,
        //    recurse into its subtree
        //    - `sw.os debian` has direct requirement type `hw.device-type`
        //      → not disjoint → evaluate, and the check must find
        //      `artik10` two subtrees and two levels away
        // - `group-2` similarly disjoint → skip, recurse
        //    - `sub` disjoint → skip, recurse
        //      - `hw.device-type artik10` has no requires → trivially
        //        satisfied
        //
        // If `root_children` were ever scoped to the current walk
        // level, the cross-subtree lookup would fail. This test
        // locks in the cross-subtree visibility.
        let mut container = contract(json!({
            "type": "foo",
            "slug": "bar",
            "children": {
                "group": {
                    "group-1": {
                        "type": "group",
                        "slug": "group-1",
                        "children": {
                            "sw.os": {
                                "type": "sw.os",
                                "slug": "debian",
                                "requires": [
                                    {"type": "hw.device-type", "slug": "artik10"}
                                ]
                            }
                        }
                    },
                    "group-2": {
                        "type": "group",
                        "slug": "group-2",
                        "children": {
                            "subgroup": {
                                "type": "subgroup",
                                "slug": "sub",
                                "children": {
                                    "hw.device-type": {
                                        "type": "hw.device-type",
                                        "slug": "artik10"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }));
        let allowed: &[&str] = &["hw.device-type"];
        assert!(container.are_children_satisfied(Some(allowed)));
    }
}
