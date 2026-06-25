pub mod vir_loader;
pub mod kani_generator;
pub mod adversarial_generator;
pub mod runtime_bridge;
pub mod generated;

use std::fs;

/// Compile a VIR file into generated artifacts.
pub fn compile(vir_path: &str) {
    let vir = fs::read_to_string(vir_path).expect("failed to read VIR file");

    let kani = kani_generator::generate(&vir);
    let tests = adversarial_generator::generate(&vir);

    fs::write("generated/generated_kani.rs", kani).expect("failed to write generated Kani code");
    fs::write("generated/generated_tests.rs", tests).expect("failed to write generated tests");
}
