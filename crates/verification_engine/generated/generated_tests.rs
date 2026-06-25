// Generated adversarial tests

use adversarial_runtime::*;
use verification_engine::runtime_bridge::evaluate_invariant;

#[test]
fn test_invariant_telemetry_validity_nan_injection() {
    let state = generate_corrupt_state("nan_injection");
    assert!(evaluate_invariant("INVARIANT_TELEMETRY_VALIDITY", &state));
}

#[test]
fn test_invariant_telemetry_validity_timestamp_wrap() {
    let state = generate_corrupt_state("timestamp_wrap");
    assert!(evaluate_invariant("INVARIANT_TELEMETRY_VALIDITY", &state));
}

