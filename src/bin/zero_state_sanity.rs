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

use m_v_r_esprint1::fiel::latch_halt_log;
use m_v_r_esprint1::zero_state::{execute_with_first_tick, execute_zero_state_suite};

fn main() {
    println!("
");
    println!("╔════════════════════════════════════════════════════╗");
    println!("║   ZERO-STATE SANITY CHECK - SPRINT 1 VALIDATION   ║");
    println!("║   (Final structural integrity before Phase 2)     ║");
    println!("╚════════════════════════════════════════════════════╝");

    println!("
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("PHASE 1: True Zero Boot");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let suite_zero = execute_zero_state_suite();
    suite_zero.report();

    if !suite_zero.all_passed() {
        eprintln!("
✗ Zero-state boot failed.");
        eprintln!("Cannot proceed to Phase 2 with latent kernel issues.
");
        // transition to emergency rather than exit
        eprintln!("Kernel state -> Emergency
");
        return;
    }

    println!("✓ Zero-state boot integrity confirmed.
");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("PHASE 2: First Tick Execution (σ=50)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut node = m_v_r_esprint1::tlbss_types::SubstrateNode::new(0);

    match execute_with_first_tick(&mut node, 50) {
        Ok(suite_tick1) => {
            suite_tick1.report();

            if !suite_tick1.all_passed() {
                eprintln!("
✗ First tick execution failed.");
                eprintln!("Kernel state corruption detected.
");
                eprintln!("Kernel state -> Emergency
");
                return;
            }

            println!("✓ First tick execution integrity confirmed.
");
        }
        Err(halt) => {
            eprintln!("
✗ First tick execution halted unexpectedly");
            eprintln!("   Axis: {:?}
", halt.axis);
            eprintln!("   Message: {}", halt.message);

            latch_halt_log(&halt);
            eprintln!("
Cannot proceed to Phase 2.
");
            eprintln!("Kernel state -> Emergency
");
            return;
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("FINAL VERDICT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("
✓ ZERO-STATE SANITY CHECK: PASSED");
    println!("
The single-entity kernel exhibits no latent ambiguities");
    println!("under fragile zero-state conditions.");
    println!("
Final state after Tick 0:");
    println!("  charge:       {}", node.charge);
    println!("  stable_ticks: {}", node.stable_ticks);
    println!("  masked_signal: 0x{:02X}", node.masked_signal);
    println!("
✓ Ready for Phase 2: Multi-Entity Scaling
");
}
