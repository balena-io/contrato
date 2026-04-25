# contrato

A contract system for describing composable, versioned things and their relationships.

Contracts represent versioned "things" (devices, operating systems, software stacks, etc.) with typed relationships expressed through requirements and capabilities. This crate provides the core engine for constructing, hashing, searching, and validating contracts.

## Usage

Add `contrato` to your `Cargo.toml`:

```toml
[dependencies]
contrato = { git = "https://github.com/balena-io/contrato" }
```

### Creating contracts

Contracts are constructed by deserializing JSON into a `Contract`. Construction automatically processes the children tree, interpolates `{{this.*}}` templates, builds the requirements index, and lazily computes a deterministic hash.

```rust
use contrato::Contract;

let contract: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "balenaos",
    "version": "6.1.2",
    "children": [
        { "type": "sw.service", "slug": "balena-engine", "version": "20.10.43" },
        { "type": "sw.service", "slug": "NetworkManager", "version": "0.6.0" }
    ]
})).unwrap();

assert_eq!(contract.get_type(), "sw.os");
assert_eq!(contract.get_slug(), Some("balenaos"));
assert_eq!(contract.get_children_types(), vec!["sw.service"]);
```

### Searching for children

Use `ContractMatcher` to find children by type, slug, version, or data fields. Results are cached internally by matcher hash.

```rust
use contrato::{Contract, ContractMatcher, ContractType, Slug, VersionReq};

let mut os: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "balenaos",
    "version": "6.1.2",
    "children": [
        { "type": "sw.service", "slug": "balena-engine", "version": "20.10.43" },
        { "type": "sw.service", "slug": "NetworkManager", "version": "0.6.0" }
    ]
})).unwrap();

// Find all sw.service children with version >= 20
let matcher = ContractMatcher::new(
    ContractType::new("sw.service"),
    None,
    Some(VersionReq::new(">=20")),
    None,
);
let matches = os.find_children(&matcher);
assert_eq!(matches.len(), 1);
assert_eq!(matches[0].get_slug(), Some("balena-engine"));
```

### Requirement satisfaction

Contracts can declare requirements via `requires`. The satisfaction engine checks whether a parent contract's children fulfill a child contract's requirements.

```rust
use contrato::Contract;

let mut parent: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "balenaos",
    "children": [
        { "type": "sw.library", "slug": "glibc", "version": "2.31.0" }
    ]
})).unwrap();

let app: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.application",
    "slug": "myapp",
    "requires": [
        { "type": "sw.library", "slug": "glibc", "version": ">=2.17.0" }
    ]
})).unwrap();

assert!(parent.satisfies_child_contract(&app, None));
```

Requirements support `or` and `not` combinators:

```rust
use contrato::Contract;

let mut parent: Contract = serde_json::from_value(serde_json::json!({
    "type": "hw.board",
    "slug": "rpi4",
    "children": [
        { "type": "arch.sw", "slug": "aarch64" }
    ]
})).unwrap();

let stack: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.stack",
    "slug": "node",
    "requires": [
        { "or": [
            { "type": "arch.sw", "slug": "aarch64" },
            { "type": "arch.sw", "slug": "amd64" }
        ]}
    ]
})).unwrap();

assert!(parent.satisfies_child_contract(&stack, None));
```

### Variant expansion

A contract with `variants` expands into multiple concrete contracts. Each variant is deep-merged with the base, and variants can nest recursively.

```rust
use contrato::{Contract, RawContract};

let source: RawContract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "alpine",
    "variants": [
        { "version": "3.19" },
        { "version": "3.20" }
    ]
})).unwrap();

let contracts = Contract::build(&source);
assert_eq!(contracts.len(), 2);
assert_eq!(contracts[0].get_version(), Some("3.19".to_string()));
assert_eq!(contracts[1].get_version(), Some("3.20".to_string()));
```

### Universe

A `Universe` is a contract of type `meta.universe` that serves as the root container for a collection of contracts. It derefs to `Contract`, so all contract methods are available.

```rust
use contrato::{Contract, Universe};

let mut universe = Universe::new();

let os: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "debian",
    "version": "12"
})).unwrap();

universe.add_child(os);
assert_eq!(universe.get_children().len(), 1);
```

## Design

- **Lazy hashing**: contracts compute their SHA-256 hash on first access and cache it. Mutations invalidate the cache. This avoids hashing ephemeral contracts that are discarded before the hash is ever read.
- **Search caching**: `find_children` results are cached by `(target_type, matcher_hash)` and invalidated per-type when children are added or removed.
- **Template interpolation**: string values containing `{{this.path}}` placeholders are resolved against the contract's own fields at construction time.

## License

Apache-2.0
