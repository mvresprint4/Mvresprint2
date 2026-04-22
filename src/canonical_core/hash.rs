#![deny(unsafe_code)]

use crate::canonical_core::serialize::canonical_record_string_utf8;
use crate::sced_offer_chain::ScedResourceOfferRecord;
use sha2::{Digest, Sha256};

pub(crate) fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub(crate) fn record_hash_hex(record: &ScedResourceOfferRecord) -> String {
    sha256_hex(&canonical_record_string_utf8(record))
}

pub(crate) fn chain_hash_hex(previous_chain_hash: &str, record_hash: &str) -> String {
    sha256_hex(&format!("{}|{}", previous_chain_hash, record_hash))
}
