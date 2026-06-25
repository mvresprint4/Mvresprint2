#![allow(dead_code)]

pub fn inject_nan() -> f64 {
    f64::NAN
}

pub fn corrupt_timestamp(ts: u64) -> u64 {
    ts.wrapping_add(999_999)
}

pub fn generate_corrupt_state(scenario: &str) -> String {
    format!("corrupt-state: {}", scenario)
}
