# Phase III Evidence Index

## Purpose
One master list of evidence generated during Phase I and Phase II. Each entry includes the artifact path, description, and how it is intended to be used for audit validation.

| Evidence Artifact | Location | Description | Source Type | Reproducibility | Linked Claims |
|---|---|---|---|---|---|
| Demo run log | `/tmp/phase_ii_repro/logs/demo_run.log` | Output from `cargo run --bin demo -- normal` | runtime | non-reproducible (external/tmp) | `demo` |
| Pilot attestation logs | `phase_ii/determinism/pilot_attestation_log_run_1.json` through `pilot_attestation_log_run_10.json` | Repeated deterministic attestation outputs for `pilot_demo` | runtime | reproducible (repo-local) | `pilot_demo`, `Deterministic evidence artifact` |
| Verifier run log | `/tmp/phase_ii_repro/logs/verifier_run.log` | Execution output from `cargo run --bin verifier pilot_attestation_log.json` | runtime | non-reproducible (external/tmp) | `verifier` |
| Formal proof harness log | `/tmp/phase_ii_repro/logs/formal_proof_harness_run.log` | Output from `cargo run --bin formal_proof_harness` | runtime | non-reproducible (external/tmp) | `formal_proof_harness` |
| Adversarial invalid payloads | `phase_ii/adversarial/*.json` | Collection of malformed and invalid attestation artifacts | test/doc | reproducible (repo-local) | `verifier`, `Attestation format contract` |
| Determinism artifacts | `phase_ii/determinism/*.json` | Repeated attestation logs used for SHA256 comparison | runtime | reproducible (repo-local) | `pilot_demo`, `Deterministic evidence artifact` |
| Branch analysis reports | `BRANCH_ANALYSIS_PHASE1.md`, `BRANCH_ANALYSIS_PHASE2.md` | Phase analysis and planning documentation | doc | reproducible (repo-local) | `Independent audit readiness`, `MERGE_PLAN` |
| Integration plan | `MERGE_PLAN.md` | Phase 3 merge execution plan and preconditions | doc | reproducible (repo-local) | `Independent audit readiness` |
| Readiness assessment | `phase_ii/readiness_assessment.md` | Phase II assessment of reproducibility and readiness | doc | reproducible (repo-local) | `Independent audit readiness` |
| Determinism report | `phase_ii/phase_ii_determinism_report.md` | Phase II determinism analysis | doc | reproducible (repo-local) | `Deterministic evidence artifact` |
| Capability matrix | `phase_ii/capability_traceability_matrix.md` | Traceability of operational claims in Phase II | doc | reproducible (repo-local) | multiple claims |
| Invariant register | `phase_ii/invariant_register.md` | Verified invariants and enforcement mechanisms | doc | reproducible (repo-local) | multiple claims/invariants |
| Verifier failure matrix | `phase_ii/verifier_failure_matrix.md` | Documented failure cases and inputs for the verifier | doc/test | reproducible (repo-local) | `verifier` |
| Reproduction logs | `phase_ii/reproduction_logs/` | Logs produced during Phase II reproduction exercises | runtime/log | reproducible (repo-local if checked in) | various claims |

## Enforcement notes
- Artifacts located under `/tmp/...` are treated as non-reproducible external artifacts and therefore are classified as `ORPHANED` unless mirrored into the repository.
- All repo-local artifacts should be linked to one or more claims. Any artifact without linkage will be reported as `ORPHANED` by the enforcement engine.

## Notes
- If an artifact referenced here is missing, it must be marked as `MISSING` and the audit framework must note why.
- The evidence index is intentionally broad: it includes both raw artifacts and documentation that supports audit claims.
- Future Phase III work should preserve the provenance of each artifact and update this index when new evidence is generated.
