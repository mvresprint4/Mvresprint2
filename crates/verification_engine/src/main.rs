use std::env;

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("compile") => {
            let vir_path = args.next().expect("expected VIR path");
            verification_engine::compile(&vir_path);
        }
        _ => {
            eprintln!("Usage: verification_engine compile <vir-path>");
            std::process::exit(1);
        }
    }
}
