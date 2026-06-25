use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Vir {
    pub invariants: Vec<Invariant>,
}

#[derive(Debug, Deserialize)]
pub struct Invariant {
    pub id: String,
    pub inputs: String,
    pub kani: KaniClause,
    pub adversarial: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct KaniClause {
    pub assumes: Vec<String>,
    pub asserts: Vec<String>,
}

pub fn load(vir: &str) -> Vir {
    serde_json::from_str(vir).expect("failed to parse VIR")
}
