// Copyright (c) 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System.

use m_v_r_esprint1::sced_offer_chain::{
    verify_truth_package, CanonicalTruthPackage, VerifyCode, VerifierReport,
};
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(
            "Usage: cargo run --bin verifier -- <canonical_truth.json> [expected_final_chain_hash]"
                .to_string(),
        );
    }

    let path = &args[1];
    let expected_hash = args.get(2).map(String::as_str);

    let data = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read canonical truth package '{}': {}", path, e))?;

    let truth: CanonicalTruthPackage = serde_json::from_str(&data)
        .map_err(|e| format!("Invalid canonical truth package JSON '{}': {}", path, e))?;

    let report = verify_truth_package(&truth, expected_hash);
    emit_json(&report)?;

    if report.status == "PASS" {
        println!(
            "[PASS] verification_complete records_verified={} final_chain_hash={}",
            report.records_verified, report.final_chain_hash
        );
    } else if let Some(first) = report.errors.first() {
        println!(
            "[FAIL] code={} mismatch_index={} key={}",
            code_as_str(&first.code),
            report
                .mismatch_index
                .map(|x| x.to_string())
                .unwrap_or_default(),
            format_key(&first.record_key)
        );
    }

    Ok(())
}

fn emit_json(report: &VerifierReport) -> Result<(), String> {
    let json = serde_json::to_string(report).map_err(|e| format!("JSON encode failed: {e}"))?;
    println!("{json}");
    Ok(())
}

fn format_key(key: &m_v_r_esprint1::sced_offer_chain::RecordKey) -> String {
    format!(
        "({},{},{},{})",
        key.scd_timestamp, key.repeat_hour_flag, key.resource_name, key.offer_type
    )
}

fn code_as_str(code: &VerifyCode) -> &'static str {
    match code {
        VerifyCode::DuplicatePk => "DUPLICATE_PK",
        VerifyCode::InvalidNumeric => "INVALID_NUMERIC",
        VerifyCode::InvalidBoolean => "INVALID_BOOLEAN",
        VerifyCode::HashMismatch => "HASH_MISMATCH",
        VerifyCode::CsvSchemaMismatch => "CSV_SCHEMA_MISMATCH",
        VerifyCode::RecordCountMismatch => "RECORD_COUNT_MISMATCH",
        VerifyCode::ChainContinuityBreak => "CHAIN_CONTINUITY_BREAK",
        VerifyCode::CsvMalformed => "CSV_MALFORMED",
    }
}
