use crate::vir_loader::load;

pub fn generate(vir: &str) -> String {
    let data = load(vir);
    let mut out = String::new();

    out.push_str("// Generated Kani proofs\n");
    for inv in data.invariants {
        let id_lower = inv.id.to_lowercase();
        out.push_str(&format!("#[cfg(kani)]\n#[kani::proof]\nfn proof_{}() {{\n", id_lower));
        out.push_str(&format!("    #[cfg(kani)] let input: {} = kani::any();\n", inv.inputs));

        for a in inv.kani.assumes {
            out.push_str(&format!("    kani_bindings::assume({});\n", a));
        }

        for a in inv.kani.asserts {
            out.push_str(&format!("    kani_bindings::assert({});\n", a));
        }

        out.push_str("}\n\n");
    }

    out
}
