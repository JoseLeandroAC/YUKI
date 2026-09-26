use yuki::capabilities::echo::EchoCapability;
use yuki::capabilities::registry::{CapabilityHandler, CapabilityRegistry};
use yuki::contracts::capability::{RiskClass, SideEffects};
use yuki::contracts::identifiers::CapabilityId;

#[test]
fn test_a_valid_echo_capability_works() {
    let echo = EchoCapability::new();
    let manifest = echo.manifest();

    // Verify manifest specifications
    assert_eq!(manifest.id, CapabilityId::new("system.echo"));
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.risk_class, RiskClass::Low);
    assert_eq!(manifest.side_effects, SideEffects::None);
    assert!(!manifest.network_required);
    assert!(!manifest.filesystem_required);
    assert!(!manifest.secrets_required);

    // Verify execution
    let input = serde_json::json!({
        "message": "Mensagem de teste determinística"
    });
    let result = echo.execute(&input).expect("echo execution should succeed");
    assert_eq!(
        result.get("echoed_message").and_then(|v| v.as_str()),
        Some("Mensagem de teste determinística")
    );
}

#[test]
fn test_echo_registry_registration() {
    let registry = CapabilityRegistry::new();
    let cap_id = CapabilityId::new("system.echo");

    assert!(registry.has_capability(&cap_id));
    let manifest = registry.get_manifest(&cap_id).expect("manifest must exist");
    assert_eq!(manifest.id, cap_id);

    let handler = registry.get_handler(&cap_id).expect("handler must exist");
    let input = serde_json::json!({ "message": "hello" });
    let output = handler.execute(&input).expect("handler execution");
    assert_eq!(output["echoed_message"], "hello");
}
