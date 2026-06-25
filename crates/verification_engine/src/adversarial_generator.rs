use crate::vir_loader::load;

pub fn generate(vir: &str) -> String {
    let data = load(vir);
    let mut out = String::new();

    out.push_str("// Generated adversarial tests\n");
    out.push_str("use adversarial_runtime::*;\n");
    out.push_str("use verification_engine::runtime_bridge::evaluate_invariant;\n\n");

    for inv in data.invariants {
        let id_lower = inv.id.to_lowercase();
        for test in inv.adversarial {
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
