// Copyright (c) 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System.

use m_v_r_esprint1::sced_offer_chain::compute_canonical_truth;
use std::env;
use std::fs::File;
use std::io::Write;
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
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args[1] != "propose" {
        return Err(
            "Usage: cargo run --bin sced_chain -- propose <input.csv> [output.json]".to_string(),
        );
    }

    let input_path = &args[2];
    let output_path = args.get(3);

    let file = File::open(input_path)
        .map_err(|e| format!("Failed to open input CSV '{}': {}", input_path, e))?;

    let truth = compute_canonical_truth(file).map_err(map_parse_error_to_string)?;
    let json = serde_json::to_string_pretty(&truth)
        .map_err(|e| format!("Failed to encode canonical truth package: {e}"))?;

    if let Some(path) = output_path {
        let mut out = File::create(path)
            .map_err(|e| format!("Failed to create output file '{}': {}", path, e))?;
        out.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write output file '{}': {}", path, e))?;
        println!(
            "[OK] canonical_truth_package_written path={} records={} final_chain_hash={}",
            path, truth.metadata.records_total, truth.final_chain_hash
        );
    } else {
        println!("{json}");
    }

    Ok(())
}

fn map_parse_error_to_string(err: m_v_r_esprint1::sced_offer_chain::ParseError) -> String {
    match err {
        m_v_r_esprint1::sced_offer_chain::ParseError::CsvSchemaMismatch => {
            "CSV schema mismatch".to_string()
        }
        m_v_r_esprint1::sced_offer_chain::ParseError::MissingValue(field) => {
            format!("missing value for field '{field}'")
        }
        m_v_r_esprint1::sced_offer_chain::ParseError::InvalidBoolean(v) => {
            format!("invalid boolean '{v}'")
        }
        m_v_r_esprint1::sced_offer_chain::ParseError::InvalidNumeric(field, v) => {
            format!("invalid numeric field '{field}' with value '{v}'")
        }
        m_v_r_esprint1::sced_offer_chain::ParseError::DuplicatePrimaryKey(ts, rep, res, typ) => {
            format!("duplicate primary key detected ({ts},{rep},{res},{typ})")
        }
        m_v_r_esprint1::sced_offer_chain::ParseError::MalformedCsv(msg) => msg,
    }
}
