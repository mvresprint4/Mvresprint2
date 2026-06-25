use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Vir {
    invariants: Vec<Invariant>,
}

#[derive(Debug, Deserialize)]
struct Invariant {
    id: String,
    inputs: String,
    kani: KaniClause,
    adversarial: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct KaniClause {
    assumes: Vec<String>,
    asserts: Vec<String>,
}

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let vir_path = Path::new(&manifest_dir).join("../../rts_kani_vir.json");
    let generated_dir = Path::new(&manifest_dir).join("generated");

    fs::create_dir_all(&generated_dir).expect("failed to create generated directory");

    let vir = fs::read_to_string(&vir_path).expect("failed to read VIR file");
    let data: Vir = serde_json::from_str(&vir).expect("failed to parse VIR");

    let kani_code = generate_kani(&data);
    let tests_code = generate_tests(&data);

    fs::write(generated_dir.join("generated_kani.rs"), kani_code).expect("failed to write generated Kani code");
    fs::write(generated_dir.join("generated_tests.rs"), tests_code).expect("failed to write generated tests");
}

fn generate_kani(data: &Vir) -> String {
    let mut out = String::new();
    out.push_str("// Generated Kani proofs\n\n");

    for inv in &data.invariants {
        let id_lower = inv.id.to_lowercase();
        out.push_str(&format!("#[cfg(kani)]\n#[kani::proof]\nfn proof_{}() {{\n", id_lower));
        out.push_str(&format!("    #[cfg(kani)] let input: {} = kani::any();\n", inv.inputs));

        for a in &inv.kani.assumes {
            out.push_str(&format!("    kani_bindings::assume({});\n", a));
        }
        for a in &inv.kani.asserts {
            out.push_str(&format!("    kani_bindings::assert({});\n", a));
        }

        out.push_str("}\n\n");
    }

    out
}

fn generate_tests(data: &Vir) -> String {
    let mut out = String::new();
    out.push_str("// Generated adversarial tests\n\n");
    out.push_str("use adversarial_runtime::*;\n");
    out.push_str("use verification_engine::runtime_bridge::evaluate_invariant;\n\n");

    for inv in &data.invariants {
        let id_lower = inv.id.to_lowercase();
        for test in &inv.adversarial {
            out.push_str(&format!(
                "#[test]\nfn test_{}_{}() {{\n",
                id_lower, test
            ));
            out.push_str(&format!(
                "    let state = generate_corrupt_state(\"{}\");\n",
                test
            ));
            out.push_str(&format!(
                "    assert!(evaluate_invariant(\"{}\", &state));\n",
                inv.id
            ));
            out.push_str("}\n\n");
        }
    }

    out
}
