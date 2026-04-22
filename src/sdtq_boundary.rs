#![deny(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardedOperation {
    ReadOnlyObserve,
    TruthComputation,
}

pub trait WitnessOnly {
    fn binary_id(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy)]
pub struct WitnessRuntimeGuard {
    binary: &'static str,
}

impl WitnessRuntimeGuard {
    pub fn new(binary: &'static str) -> Self {
        Self { binary }
    }
}

impl WitnessOnly for WitnessRuntimeGuard {
    fn binary_id(&self) -> &'static str {
        self.binary
    }
}

pub fn enforce_witness_only<T: WitnessOnly>(
    witness: &T,
    operation: GuardedOperation,
) -> Result<(), String> {
    match operation {
        GuardedOperation::ReadOnlyObserve => Ok(()),
        GuardedOperation::TruthComputation => Err(format!(
            "SDTQ design violation: witness binary '{}' attempted truth computation",
            witness.binary_id()
        )),
    }
}
