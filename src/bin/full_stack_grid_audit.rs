// Copyright (c) 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System.

use m_v_r_esprint1::sced_offer_chain::{CanonicalTruthPackage, VerifierReport};
use m_v_r_esprint1::sovereign_diagnostic::{
    default_simulation_signing_key, emit_signed_diagnostic,
};
use m_v_r_esprint1::sovereign_trace::seal_verified_truth_commit;
use base64::Engine;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
struct AuditCommitEntry {
    committed_at_unix_ms: u128,
    verifier_status: String,
    final_chain_hash: String,
    records_total: usize,
    schema_version: String,
    hash_spec_version: String,
    verifier_mismatch_index: Option<usize>,
    verifier_errors_total: usize,
    sovereign_seal_hash: String,
    sovereign_seal_signature: String,
    sovereign_seal_payload: String,
    sda_dtc: String,
    sda_diagnostic_b64: String,
    sda_artifact_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err(
            "Usage: cargo run --bin full_stack_grid_audit -- <canonical_truth.json> <verifier_report.json> [audit_log.jsonl]"
                .into(),
        );
    }

    let truth_path = &args[1];
    let report_path = &args[2];
    let output_log = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "artifacts/verified_truth_audit_log.jsonl".to_string());

    let truth: CanonicalTruthPackage = serde_json::from_str(&fs::read_to_string(truth_path)?)?;
    let report: VerifierReport = serde_json::from_str(&fs::read_to_string(report_path)?)?;

    if report.status != "PASS" {
        return Err("Audit historian refusal: verifier status is not PASS".into());
    }
    if report.final_chain_hash != truth.final_chain_hash {
        return Err("Audit historian refusal: verifier hash does not match canonical truth hash".into());
    }
    let seal = seal_verified_truth_commit(
        &truth.final_chain_hash,
        &report.status,
        truth.metadata.records_total,
        &truth.metadata.schema_version,
        &truth.metadata.hash_spec_version,
    )
    .map_err(|e| format!("{:?}: {}", e.axis, e.message))?;

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let firmware_hash = hash_firmware_bytes()?;
    let signing_key = default_simulation_signing_key();
    let diagnostic = emit_signed_diagnostic(
        &truth.canonical_payload_bytes,
        firmware_hash,
        &report,
        &signing_key,
    )
    .map_err(|e| format!("SDA emit failed: {e}"))?;

    let diagnostic_bytes = bincode::serialize(&diagnostic)?;
    let sda_artifact_path = "artifacts/sovereign_diagnostic.bin".to_string();
    if let Some(parent) = Path::new(&sda_artifact_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&sda_artifact_path, &diagnostic_bytes)?;
    let sda_b64 = base64::engine::general_purpose::STANDARD.encode(&diagnostic_bytes);

    let entry = AuditCommitEntry {
        committed_at_unix_ms: now_ms,
        verifier_status: report.status,
        final_chain_hash: truth.final_chain_hash,
        records_total: truth.metadata.records_total,
        schema_version: truth.metadata.schema_version,
        hash_spec_version: truth.metadata.hash_spec_version,
        verifier_mismatch_index: report.mismatch_index,
        verifier_errors_total: report.errors.len(),
        sovereign_seal_hash: seal.seal_hash_hex,
        sovereign_seal_signature: seal.seal_signature_hex,
        sovereign_seal_payload: seal.payload,
        sda_dtc: diagnostic.dtc.clone(),
        sda_diagnostic_b64: sda_b64,
        sda_artifact_path,
    };

    if let Some(parent) = std::path::Path::new(&output_log).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&output_log)?;

    let line = serde_json::to_string(&entry)?;
    writeln!(file, "{}", line)?;

    println!("Historian commit: PASS");
    println!("Committed chain hash: {}", entry.final_chain_hash);
    println!("SDA DTC: {}", entry.sda_dtc);
    println!("Audit log: {}", output_log);

    Ok(())
}

fn hash_firmware_bytes() -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let path = std::env::var("MVRE_FIRMWARE_PATH")
        .unwrap_or_else(|_| "target/release/sced_chain".to_string());
    let bytes = fs::read(&path)
        .map_err(|e| format!("firmware hash read failed for '{}': {}", path, e))?;
    Ok(Sha256::digest(&bytes).into())
}
