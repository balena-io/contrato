//! Validates that the code examples in README.md compile and pass.

use contrato::{Contract, Matcher, RawContract, Universe};

#[test]
fn readme_quickstart() {
    let os_contract: Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "balenaos",
        "version": "6.1.2",
        "children": [
            { "type": "sw.service", "slug": "balena-engine", "version": "20.10.43" },
            { "type": "sw.service", "slug": "NetworkManager", "version": "0.6.0" },
            { "type": "sw.feature", "slug": "secureboot" }
        ]
    }))
    .unwrap();

    let service_contract: Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.application",
        "slug": "myapp",
        "requires": [
            { "type": "sw.service", "slug": "balena-engine", "version": ">=20" },
            { "type": "sw.feature", "slug": "secureboot" }
        ]
    }))
    .unwrap();

    assert!(os_contract.satisfies_child_contract(&service_contract, None));
}

#[test]
fn readme_creating_contracts() {
    let contract: Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "balenaos",
        "version": "6.1.2",
        "children": [
            { "type": "sw.service", "slug": "balena-engine", "version": "20.10.43" },
            { "type": "sw.service", "slug": "NetworkManager", "version": "0.6.0" }
        ]
    }))
    .unwrap();

    assert_eq!(contract.get_type(), "sw.os");
    assert_eq!(contract.get_slug(), Some("balenaos"));
    assert_eq!(contract.get_children_types(), vec!["sw.service"]);
}

#[test]
fn readme_searching() {
    let os: Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "balenaos",
        "version": "6.1.2",
        "children": [
            { "type": "sw.service", "slug": "balena-engine", "version": "20.10.43" },
            { "type": "sw.service", "slug": "NetworkManager", "version": "0.6.0" }
        ]
    }))
    .unwrap();

    let matcher = Matcher::new("sw.service").with_version(">=20");
    let matches = os.find_children(&matcher);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].get_slug(), Some("balena-engine"));
}

#[test]
fn readme_validation() {
    let os_contract: Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "balenaos",
        "version": "4.1.5",
        "children": [
            { "type": "sw.library", "slug": "glibc", "version": "2.16" }
        ]
    }))
    .unwrap();

    assert!(
        os_contract.satisfies_child_contract(
            &serde_json::from_value(serde_json::json!({
                "type": "sw.utility",
                "slug": "myapp",
                "version": "8.11.1",
                "requires": [{ "type": "sw.library", "slug": "glibc", "version": ">=2.15" }]
            }))
            .unwrap(),
            None,
        )
    );

    assert!(
        !os_contract.satisfies_child_contract(
            &serde_json::from_value(serde_json::json!({
                "type": "sw.utility",
                "slug": "myapp",
                "version": "8.11.1",
                "requires": [{ "type": "sw.library", "slug": "glibc", "version": "<2" }]
            }))
            .unwrap(),
            None,
        )
    );
}

#[test]
fn readme_or_combinator() {
    let board: Contract = serde_json::from_value(serde_json::json!({
        "type": "hw.board",
        "slug": "rpi4",
        "children": [
            { "type": "arch.sw", "slug": "aarch64" }
        ]
    }))
    .unwrap();

    let stack: Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.stack",
        "slug": "node",
        "requires": [
            { "or": [
                { "type": "arch.sw", "slug": "aarch64" },
                { "type": "arch.sw", "slug": "amd64" }
            ]}
        ]
    }))
    .unwrap();

    assert!(board.satisfies_child_contract(&stack, None));
}

#[test]
fn readme_not_satisfied_requirements() {
    let os_contract: Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "balenaos",
        "version": "4.1.5",
        "children": [
            { "type": "sw.library", "slug": "glibc", "version": "2.16" }
        ]
    }))
    .unwrap();

    let curl: Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.utility",
        "slug": "curl",
        "version": "8.11.1",
        "requires": [{ "type": "sw.library", "slug": "glibc", "version": ">=2.17" }]
    }))
    .unwrap();

    let missing = os_contract.get_not_satisfied_child_requirements(&curl, None);
    assert_eq!(
        serde_json::to_string(&missing).unwrap(),
        r#"[{"type":"sw.library","slug":"glibc","version":">=2.17"}]"#
    );
}

#[test]
fn readme_interpolation() {
    let contract: Contract = serde_json::from_value(serde_json::json!({
        "slug": "mycontract",
        "type": "sw.application",
        "version": "1.0.0",
        "name": "This is my contract",
        "aliases": ["my-contract"],
        "data": { "number": 5 },
        "assets": {
            "file": {
                "url": "https://files.contracts.io/{{this.slug}}/{{this.version}}/{{this.data.number}}.data"
            }
        }
    }))
    .unwrap();

    let value = serde_json::to_value(&contract).unwrap();
    assert_eq!(
        value["assets"]["file"]["url"],
        serde_json::json!("https://files.contracts.io/mycontract/1.0.0/5.data")
    );
}

#[test]
fn readme_variants() {
    let source: RawContract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "alpine",
        "variants": [
            { "version": "3.19" },
            { "version": "3.20" }
        ]
    }))
    .unwrap();

    let contracts = Contract::build(source).unwrap();

    assert_eq!(contracts.len(), 2);
    assert_eq!(contracts[0].get_version(), Some("3.19".to_string()));
    assert_eq!(contracts[1].get_version(), Some("3.20".to_string()));
}

#[test]
fn readme_universe() {
    let mut universe = Universe::new();

    universe
        .add_children(vec![
            serde_json::from_value(serde_json::json!({
                "type": "sw.os", "slug": "debian", "version": "12"
            }))
            .unwrap(),
            serde_json::from_value(serde_json::json!({
                "type": "sw.os", "slug": "alpine", "version": "3.20"
            }))
            .unwrap(),
        ])
        .unwrap();

    let children = universe.find_children(&Matcher::new("sw.os").with_slug("debian"));

    assert_eq!(children.len(), 1);
}
