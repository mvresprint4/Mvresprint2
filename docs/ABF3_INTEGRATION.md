ABF-3 Integration Guide

Purpose

ABF-3 (Adversarial Buffer Firewall) is a pre-kernel enforcement layer implemented
inside existing ingress and kernel transition paths. It is intentionally not a
separate crate: the system currently lacks a stable interconnect (YBus), so
ABF-3 must be anchored into current files.

Integration Map

- Ingress Gate (PRIMARY ENTRY POINT): `src/ai_ingestion_buffer.rs`
  - `ABF3Sanitize` trait and `ingest(frame: AdversarialFrame) -> Result<CleanFrame, ABF3Error>`
  - Rejects NaN/Inf, malformed or empty payloads, normalizes numeric edge cases

- Mutation Engine (adversarial source): `src/adversarial_harness.rs`
  - `AdversarialFrame` and `CorruptionClass`
  - Mutation helpers: `inject_nan`, `inject_inf`, `inject_structural_shuffle`, `inject_truncation`, `inject_timing_skew`

- State Authority / Enforcement Boundary: `src/mvre_kernel.rs`
  - `KernelState::semantic_equivalence(&self, other: &Self) -> bool`
  - On inequality: call `crate::sovereign_kernel::emit_halt("HALT_0xABF3")` and return `SystemHalt`

- HALT / Audit sink: `src/sovereign_kernel.rs`
  - `emit_halt(code: &str)` — write a sovereign halt notice and act as trace sink

Developer Notes (Phase 1)

1. Do not introduce a new `abf3` crate yet. Keep enforcement inside the
   ingress and kernel transition boundaries until a stable YBus is available.
2. The ingress sanitizer returns `CleanFrame` (`Setpoint`) objects that are
   safe for kernel consumption; callers should not bypass `ingest()`.
3. Kernel semantic equivalence must remain deterministic; adjust `numeric_distance`
   conservatively to avoid false positives. Tests should cover small-number
   tolerances and known edge cases (NaN/Inf propagation).
4. Violations are fatal by design: `HALT_0xABF3` is a canonical stop code and
   must be handled by higher-level operational scripts that perform trace
   collection and incident reporting.

Kani / Formalization (future)

- Add a `kani` proof module under `tests/` that instantiates arbitrary
  `AdversarialFrame` values and asserts `ingest(frame)` either returns `Ok`
  (clean setpoint) or `Err(ABF3Error)` but never produces silent NaN/Inf
  propagation into kernel state.

- Example placeholder:

```
#[kani::proof]
fn abf3_ingress_safety() {
    let frame: AdversarialFrame = kani::any();
    let _ = match ingest(frame) {
        Ok(clean) => assert!(!clean.contains_nan_or_inf()),
        Err(_) => (),
    };
}
```

Tracing and Incident Response

- On `HALT_0xABF3`, runtime wrappers should collect `scenario_attestation_log.json`,
  kernel trace, and the offending `AdversarialFrame` (if available) and store an
  evidence bundle for postmortem analysis.

Contact

For questions or suggested changes to ABF-3 integration, open an issue or
contact the kernel maintainers listed in `MAINTAINERS.md`.
