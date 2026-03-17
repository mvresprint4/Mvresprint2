#![deny(unsafe_code)]

use mvre_sprint_guardian::fiel::latch_halt_log;
use mvre_sprint_guardian::zero_state::{execute_with_first_tick, execute_zero_state_suite};

fn main() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════╗");
    println!("║   ZERO-STATE SANITY CHECK - SPRINT 1 VALIDATION   ║");
    println!("║   (Final structural integrity before Phase 2)     ║");
    println!("╚════════════════════════════════════════════════════╝");

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("PHASE 1: True Zero Boot");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let suite_zero = execute_zero_state_suite();
    suite_zero.report();

    if !suite_zero.all_passed() {
        eprintln!("\n✗ Zero-state boot failed.");
        eprintln!("Cannot proceed to Phase 2 with latent kernel issues.\n");
        // transition to emergency rather than exit
        eprintln!("Kernel state -> Emergency\n");
        return;
    }

    println!("✓ Zero-state boot integrity confirmed.\n");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("PHASE 2: First Tick Execution (σ=50)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut node = mvre_sprint_guardian::tlbss_types::SubstrateNode::new(0);

    match execute_with_first_tick(&mut node, 50) {
        Ok(suite_tick1) => {
            suite_tick1.report();

            if !suite_tick1.all_passed() {
                eprintln!("\n✗ First tick execution failed.");
                eprintln!("Kernel state corruption detected.\n");
                eprintln!("Kernel state -> Emergency\n");
                return;
            }

            println!("✓ First tick execution integrity confirmed.\n");
        }
        Err(halt) => {
            eprintln!("\n✗ First tick execution halted unexpectedly");
            eprintln!("   Axis: {:?}\n", halt.axis);
            eprintln!("   Message: {}", halt.message);

            latch_halt_log(&halt);
            eprintln!("\nCannot proceed to Phase 2.\n");
            eprintln!("Kernel state -> Emergency\n");
            return;
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("FINAL VERDICT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("\n✓ ZERO-STATE SANITY CHECK: PASSED");
    println!("\nThe single-entity kernel exhibits no latent ambiguities");
    println!("under fragile zero-state conditions.");
    println!("\nFinal state after Tick 0:");
    println!("  charge:       {}", node.charge);
    println!("  stable_ticks: {}", node.stable_ticks);
    println!("  masked_signal: 0x{:02X}", node.masked_signal);
    println!("\n✓ Ready for Phase 2: Multi-Entity Scaling\n");
}
