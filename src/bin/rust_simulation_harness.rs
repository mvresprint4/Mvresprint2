#![deny(unsafe_code)]

use mvre_sprint_guardian::simulation_harness_core::run_all;

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
