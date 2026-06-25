#![allow(dead_code)]

/// Safe wrapper around Kani proof primitives.
#[cfg(kani)]
pub fn assume(cond: bool) {
    kani::assume(cond);
}

#[cfg(not(kani))]
pub fn assume(_cond: bool) {
    // No-op outside Kani builds.
}

#[cfg(kani)]
pub fn assert(cond: bool) {
    kani::assert(cond);
}

#[cfg(not(kani))]
pub fn assert(_cond: bool) {
    // No-op outside Kani builds.
}
