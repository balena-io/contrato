//! Validates that the code examples in README.md compile and pass.

#[test]
fn readme_creating_contracts() {
    let contract: contrato::Contract = serde_json::from_value(serde_json::json!({
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
    let os: contrato::Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "balenaos",
        "version": "6.1.2",
        "children": [
            { "type": "sw.service", "slug": "balena-engine", "version": "20.10.43" },
            { "type": "sw.service", "slug": "NetworkManager", "version": "0.6.0" }
        ]
    }))
    .unwrap();
    let matcher = contrato::ContractMatcher::new(
        contrato::ContractType::new("sw.service"),
        None,
        Some(contrato::VersionReq::new(">=20")),
        None,
    );
    let matches = os.find_children(&matcher);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].get_slug(), Some("balena-engine"));
}

#[test]
fn readme_satisfaction() {
    let parent: contrato::Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "balenaos",
        "children": [
            { "type": "sw.library", "slug": "glibc", "version": "2.31.0" }
        ]
    }))
    .unwrap();
    let app: contrato::Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.application",
        "slug": "myapp",
        "requires": [
            { "type": "sw.library", "slug": "glibc", "version": ">=2.17.0" }
        ]
    }))
    .unwrap();
    assert!(parent.satisfies_child_contract(&app, None));
}

#[test]
fn readme_or_combinator() {
    let parent: contrato::Contract = serde_json::from_value(serde_json::json!({
        "type": "hw.board",
        "slug": "rpi4",
        "children": [
            { "type": "arch.sw", "slug": "aarch64" }
        ]
    }))
    .unwrap();
    let stack: contrato::Contract = serde_json::from_value(serde_json::json!({
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
    assert!(parent.satisfies_child_contract(&stack, None));
}

#[test]
fn readme_variants() {
    let source: contrato::RawContract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "alpine",
        "variants": [
            { "version": "3.19" },
            { "version": "3.20" }
        ]
    }))
    .unwrap();
    let contracts = contrato::Contract::build(&source);
    assert_eq!(contracts.len(), 2);
    assert_eq!(contracts[0].get_version(), Some("3.19".to_string()));
    assert_eq!(contracts[1].get_version(), Some("3.20".to_string()));
}

#[test]
fn readme_universe() {
    let mut universe = contrato::Universe::new();
    let os: contrato::Contract = serde_json::from_value(serde_json::json!({
        "type": "sw.os",
        "slug": "debian",
        "version": "12"
    }))
    .unwrap();
    universe.add_child(os);
    assert_eq!(universe.get_children().len(), 1);
}
