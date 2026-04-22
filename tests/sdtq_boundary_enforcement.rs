use std::fs;
use std::path::{Path, PathBuf};

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
}

#[test]
fn canonical_core_references_are_isolated() {
    let mut files = Vec::new();
    collect_rs_files(Path::new("src"), &mut files);

    for path in files {
        let rel = path.to_string_lossy().replace('\\', "/");
        let content = fs::read_to_string(&path).unwrap_or_default();

        let is_canonical_core = rel.starts_with("src/canonical_core/");
        let is_sced_offer_chain = rel == "src/sced_offer_chain.rs";
        if !is_canonical_core && !is_sced_offer_chain {
            assert!(
                !content.contains("canonical_core::"),
                "Forbidden canonical_core reference outside core boundary: {}",
                rel
            );
        }
    }
}

#[test]
fn only_sced_chain_may_compute_canonical_truth() {
    let mut files = Vec::new();
    collect_rs_files(Path::new("src/bin"), &mut files);

    for path in files {
        let rel = path.to_string_lossy().replace('\\', "/");
        let content = fs::read_to_string(&path).unwrap_or_default();
        let is_sced_chain = rel == "src/bin/sced_chain.rs";
        if !is_sced_chain {
            assert!(
                !content.contains("compute_canonical_truth("),
                "Only sced_chain may compute canonical truth. Found in {}",
                rel
            );
        }
    }
}
