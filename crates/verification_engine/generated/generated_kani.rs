// Generated Kani proofs

#[cfg(kani)]
#[kani::proof]
fn proof_invariant_telemetry_validity() {
    #[cfg(kani)] let input: rts_invariants::TelemetryFrame = kani::any();
    kani_bindings::assume(input.p.is_finite());
    kani_bindings::assume(input.q.is_finite());
    kani_bindings::assert(!input.p.is_nan());
    kani_bindings::assert(!input.q.is_nan());
}

