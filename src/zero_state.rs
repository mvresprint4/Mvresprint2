#![deny(unsafe_code)]

#[derive(Debug, Clone, Copy)]
pub struct ZeroState {
    pub value: u32,
}

impl Default for ZeroState {
    fn default() -> Self {
        Self { value: 0 }
    }
}

impl ZeroState {
    pub fn new(value: u32) -> Self {
        Self { value }
    }
}
