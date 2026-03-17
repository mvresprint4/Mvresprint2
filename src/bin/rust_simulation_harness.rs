// Copyright © 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System,
// including TLBSS geometry, the Universal Execution Layer, the
// Deterministic IR, Rust Codegen Pipeline, SovereignBus, and the
// Cryptographic Audit Chain.
//
// No part of this file, its algorithms, structures, or designs may be
// copied, reproduced, modified, distributed, published, sublicensed,
// reverse-engineered, or used to create derivative works without the
// express written permission of OBINNA JAMES EJIOFOR.
//
// This software contains proprietary trade secrets and confidential
// intellectual property. Unauthorized use is strictly prohibited.


#![deny(unsafe_code)]

use m_v_r_esprint1::simulation_harness_core::run_all;

fn main() {
    let manifest_path = "artifacts/interface_commissioning_manifest.json";
    println!("Running shadow simulation harness...");
    match run_all(manifest_path) {
        Ok(()) => {
            println!("Simulation harness passed.");
        }
        Err(e) => {
            eprintln!("Simulation harness failed: {e}");
            std::process::exit(1);
        }
    }
}
