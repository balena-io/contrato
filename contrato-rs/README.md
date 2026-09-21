# contrato

A simple, yet effective system for capability/requirements description and validation via [contracts](#about-contracts) in Rust.

## Quickstart

Add `contrato` to your `Cargo.toml`:

```toml
[dependencies]
contrato = "0"
serde_json = "1"
```

```rust
use contrato::Contract;

let os_contract: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "balenaos",
    "version": "6.1.2",
    "children": [
        { "type": "sw.service", "slug": "balena-engine", "version": "20.10.43" },
        { "type": "sw.service", "slug": "NetworkManager", "version": "0.6.0" },
        { "type": "sw.feature", "slug": "secureboot" }
    ]
})).unwrap();

let service_contract: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.application",
    "slug": "myapp",
    "requires": [
        { "type": "sw.service", "slug": "balena-engine", "version": ">=20" },
        { "type": "sw.feature", "slug": "secureboot" }
    ]
})).unwrap();

if os_contract.satisfies_child_contract(&service_contract, None) {
    println!("myapp can be installed!");
}
```

## About contracts

Contracts provide a standardized mechanism to describing _things_. A thing generally refers to something versionable, e.g. a software library, a feature, an API, etc. Relationships between things can be established via composition and referencing (`children` and `requires`). Through this library, contracts can be validated, composed and combined.

### Why build this?

balena.io enables users in deploying, managing and scaling large fleets of IoT devices. These fleets may be composed from devices using different combinations of hardware and software components, as well as different OS versions. Contracts provide an interface to describe capabilities and requirements, allowing users to safely push updates to their fleets and ensure their software will only run on devices that meet the requirements to run it.

### What can be done with contracts?

Describe a _thing_

```json
{
	"type": "sw.library",
	"slug": "glibc",
	"version": "2.40",
	"assets": {
		"license": {
			"name": "GNU Lesser General Public License",
			"url": "https://www.gnu.org/licenses/lgpl-3.0.html#license-text"
		}
	}
}
```

Describe a _thing_ that requires a _thing_

```json
{
	"type": "sw.utility",
	"slug": "curl",
	"version": "8.11.1",
	"requires": [{ "type": "sw.library", "slug": "glibc", "version": ">=2.17" }],
	"data": {
		"protocols": ["HTTP", "HTTPS", "FTP"]
	}
}
```

Describe a complex _thing_ via a composite contract

```json
{
	"type": "sw.os",
	"slug": "balenaos",
	"version": "4.1.5",
	"children": [
		{
			"type": "sw.library",
			"slug": "glibc",
			"version": "2.16",
			"assets": {
				"license": {
					"name": "GNU Lesser General Public License",
					"url": "https://www.gnu.org/licenses/lgpl-3.0.html#license-text"
				}
			}
		}
	]
}
```

Children are also how a contract declares the capabilities it makes available to
its context: any child can be matched by another contract's `requires`.

Describe a set of things via [templating](#contract-templating)

```json
{
	"slug": "alpine",
	"type": "sw.os",
	"version": "1",
	"data": {
		"libc": "musl-libc",
		"latest": "3.20",
		"versionList": "`3.20 (latest)`, `3.19`"
	},
	"name": "Alpine Linux {{this.version}}",
	"requires": [{ "type": "sw.blob", "slug": "balena-idle" }],
	"variants": [
		{
			"requires": [
				{ "type": "sw.blob", "slug": "qemu" },
				{
					"or": [
						{ "type": "arch.sw", "slug": "armv7hf" },
						{ "type": "arch.sw", "slug": "rpi" },
						{ "type": "arch.sw", "slug": "aarch64" }
					]
				}
			],
			"variants": [{ "version": "3.19" }, { "version": "3.20" }]
		},
		{
			"requires": [
				{
					"or": [
						{ "type": "arch.sw", "slug": "i386" },
						{ "type": "arch.sw", "slug": "amd64" }
					]
				}
			],
			"variants": [{ "version": "3.19" }, { "version": "3.20" }]
		}
	]
}
```

## About contrato

Contrato is the Balena contracts implementation. This crate is the core contract
engine: it provides capabilities for constructing, searching, comparing
and validating contracts, as well as expanding contract templates into concrete
contracts.

Contracts are normally read from JSON, so `Contract` is constructed by
deserializing with `serde_json`. Deserialization processes the children tree,
interpolates `{{this.*}}` templates, builds the requirements index, and prepares
a lazily computed deterministic hash.

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

### Searching for contracts

The crate provides a `Matcher` type, that allows to find children by type, and optionally by slug, version or data
fields.

```rust
use contrato::{Contract, Matcher};

let os: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "balenaos",
    "version": "6.1.2",
    "children": [
        { "type": "sw.service", "slug": "balena-engine", "version": "20.10.43" },
        { "type": "sw.service", "slug": "NetworkManager", "version": "0.6.0" }
    ]
})).unwrap();

// Find all sw.service children with version >= 20
let matcher = Matcher::new("sw.service").with_version(">=20");
let matches = os.find_children(&matcher);

assert_eq!(matches.len(), 1);
assert_eq!(matches[0].get_slug(), Some("balena-engine"));
```

### Contract validation

A contract is valid within a context if all requirements of the contract and its children are met in the given context. A requirement is met if there is a contract within the context (including children) that matches the requirement. For example

```rust
use contrato::Contract;

let os_contract: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "balenaos",
    "version": "4.1.5",
    "children": [
        { "type": "sw.library", "slug": "glibc", "version": "2.16" }
    ]
})).unwrap();

// This is true
assert!(os_contract.satisfies_child_contract(
    &serde_json::from_value(serde_json::json!({
        "type": "sw.utility",
        "slug": "myapp",
        "version": "8.11.1",
        "requires": [{ "type": "sw.library", "slug": "glibc", "version": ">=2.15" }]
    })).unwrap(),
    None,
));

// This is false
assert!(!os_contract.satisfies_child_contract(
    &serde_json::from_value(serde_json::json!({
        "type": "sw.utility",
        "slug": "myapp",
        "version": "8.11.1",
        "requires": [{ "type": "sw.library", "slug": "glibc", "version": "<2" }]
    })).unwrap(),
    None,
));
```

Requirements support `or` and `not` combinators:

```rust
use contrato::Contract;

let board: Contract = serde_json::from_value(serde_json::json!({
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

assert!(board.satisfies_child_contract(&stack, None));
```

Contrato also allows to find unsatisfied requirements, e.g.

```rust
use contrato::Contract;

let os_contract: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.os",
    "slug": "balenaos",
    "version": "4.1.5",
    "children": [
        { "type": "sw.library", "slug": "glibc", "version": "2.16" }
    ]
})).unwrap();

let curl: Contract = serde_json::from_value(serde_json::json!({
    "type": "sw.utility",
    "slug": "curl",
    "version": "8.11.1",
    "requires": [{ "type": "sw.library", "slug": "glibc", "version": ">=2.17" }]
})).unwrap();

// Will print [{"type":"sw.library","slug":"glibc","version":">=2.17"}]
let missing = os_contract.get_not_satisfied_child_requirements(&curl, None);
println!("{}", serde_json::to_string(&missing).unwrap());
```

### Contract templating

String properties of contracts may reference other number or string properties declared on the same contract by using the `this` keyword along with handlebars notation.

For example

```json
{
	"slug": "mycontract",
	"type": "sw.application",
	"version": "1.0.0",
	"name": "This is my contract",
	"aliases": ["my-contract"],
	"data": {
		"number": 5
	},
	"assets": {
		"file": {
			"url": "https://files.contracts.io/{{this.slug}}/{{this.version}}/{{this.data.number}}.data"
		}
	}
}
```

These references are resolved when the contract is constructed. A contract that
is mutated afterwards can be re-resolved with `Contract::interpolate`.

A single template can be used to generate multiple contracts using the `variants` property on the contract. A good example of this is in the [Alpine OS contract template](https://github.com/balena-io/contracts/blob/master/contracts/sw.os/alpine/contract.json) describes the combination of different architecture builds for a list of versions and can be compiled into a set of contracts. See also the [NodeJS contract](https://github.com/balena-io/contracts/blob/master/contracts/sw.stack/node/contract.json) for a more complex example.

The template is compiled into concrete contracts with `Contract::build`, which
deep-merges each variant with the base contract and expands nested variants
recursively.

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

// Build the template into the resulting contracts
let contracts = Contract::build(source).unwrap();

assert_eq!(contracts.len(), 2);
assert_eq!(contracts[0].get_version(), Some("3.19".to_string()));
assert_eq!(contracts[1].get_version(), Some("3.20".to_string()));
```

### Universes

A universe is a composite contract that conforms the collection of "things" being operated on. For instance, the set of contracts on [balena-io/contracts](https://github.com/balena-io/contracts) compose the universe of Balena's contracts containing the knowledge about device types, architectures, OS versions and software stacks available to Balena and its products.

`Universe` is a contract of type `meta.universe` that serves as the root
container for such a collection. It derefs to `Contract`, so every contract
operation is available on it.

```rust
use contrato::{Contract, Matcher, Universe};

let mut universe = Universe::new();

universe.add_children(vec![
    serde_json::from_value(serde_json::json!({
        "type": "sw.os", "slug": "debian", "version": "12"
    })).unwrap(),
    serde_json::from_value(serde_json::json!({
        "type": "sw.os", "slug": "alpine", "version": "3.20"
    })).unwrap(),
]).unwrap();

// Find all contracts for the Debian OS
let children = universe.find_children(&Matcher::new("sw.os").with_slug("debian"));

assert_eq!(children.len(), 1);
```

### Limitations of contrato

Contrato is quite efficient at most tasks it performs, however most of the operations require that the validating context is stored in memory, which puts a limit to the size of the universe.

## Contribute

- Issue Tracker: [github.com/balena-io/contrato/issues](https://github.com/balena-io/contrato/issues)
- Source Code: [github.com/balena-io/contrato](https://github.com/balena-io/contrato)

Before submitting a PR, please make sure that you include tests, and that the
linter runs without any warning:

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Support

If you're having any problem, please [raise an
issue](https://github.com/balena-io/contrato/issues/new) on GitHub.

## License

The project is licensed under the Apache 2.0 license.
