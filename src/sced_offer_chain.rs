// Copyright (c) 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System,
// including TLBSS geometry, the Universal Execution Layer, the
// Deterministic IR, Rust Codegen Pipeline, SovereignBus, and the
// Cryptographic Audit Chain.
//
// No part of this file, its algorithms, structures, or designs may be
// copied, reproduced, modified, distributed, published, sublicensed,
// reverse-engineered, or used to create derivative works without the
// express written permission of OBINNA JAMES EJIOFOR.
//
// This software contains proprietary trade secrets and confidential
// intellectual property. Unauthorized use is strictly prohibited.
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use crate::canonical_core::{canonicalize, hash, serialize, sort};
use std::fmt;
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScedResourceOfferRecord {
    pub scd_timestamp: String,
    pub repeat_hour_flag: bool,
    pub resource_name: String,
    pub offer_type: String,
    #[serde(
        serialize_with = "serialize_prices_and_quantities",
        deserialize_with = "deserialize_prices_and_quantities"
    )]
    pub prices_and_quantities: [String; 48],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainedRecord {
    pub key: (String, bool, String, String),
    pub record_hash: String,
    pub chain_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    CsvSchemaMismatch,
    MissingValue(String),
    InvalidBoolean(String),
    InvalidNumeric(String, String),
    DuplicatePrimaryKey(String, bool, String, String),
    MalformedCsv(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerifyCode {
    DuplicatePk,
    InvalidNumeric,
    InvalidBoolean,
    HashMismatch,
    CsvSchemaMismatch,
    RecordCountMismatch,
    ChainContinuityBreak,
    CsvMalformed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordKey {
    pub scd_timestamp: String,
    pub repeat_hour_flag: bool,
    pub resource_name: String,
    pub offer_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyError {
    pub code: VerifyCode,
    pub message: String,
    pub record_key: RecordKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierReport {
    pub status: String,
    pub records_total: usize,
    pub records_verified: usize,
    pub final_chain_hash: String,
    pub expected_final_chain_hash: Option<String>,
    pub mismatch_index: Option<usize>,
    pub errors: Vec<VerifyError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTruthMetadata {
    pub schema_version: String,
    pub hash_spec_version: String,
    pub records_total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTruthPackage {
    pub canonical_payload_bytes: Vec<u8>,
    pub records: Vec<ScedResourceOfferRecord>,
    pub chain: Vec<ChainedRecord>,
    pub final_chain_hash: String,
    pub metadata: CanonicalTruthMetadata,
}

pub const SCED_SCHEMA_VERSION: &str = "sced.v1";
pub const SCED_HASH_SPEC_VERSION: &str = "sha256.chain.v1";

impl ScedResourceOfferRecord {
    pub fn primary_key(&self) -> (&str, bool, &str, &str) {
        (
            &self.scd_timestamp,
            self.repeat_hour_flag,
            &self.resource_name,
            &self.offer_type,
        )
    }

    pub fn canonical_record_string(&self) -> String {
        serialize::canonical_record_string_utf8(self)
    }

    pub fn record_hash(&self) -> String {
        hash::record_hash_hex(self)
    }
}

pub(crate) fn parse_csv<R: Read>(input: R) -> Result<Vec<ScedResourceOfferRecord>, ParseError> {
    canonicalize::parse_csv(input)
}

pub(crate) fn sort_records(records: &mut [ScedResourceOfferRecord]) {
    sort::sort_records(records);
}

pub(crate) fn build_hash_chain(
    records: Vec<ScedResourceOfferRecord>,
) -> Result<Vec<ChainedRecord>, ParseError> {
    canonicalize::build_hash_chain(records)
}

pub fn compute_canonical_truth<R: Read>(input: R) -> Result<CanonicalTruthPackage, ParseError> {
    let mut records = parse_csv(input)?;
    sort_records(&mut records);
    let canonical_payload_bytes = build_canonical_payload_bytes(&records);
    let chain = build_hash_chain(records.clone())?;
    let final_chain_hash = chain
        .last()
        .map(|r| r.chain_hash.clone())
        .unwrap_or_else(|| "0".to_string());

    Ok(CanonicalTruthPackage {
        canonical_payload_bytes,
        metadata: CanonicalTruthMetadata {
            schema_version: SCED_SCHEMA_VERSION.to_string(),
            hash_spec_version: SCED_HASH_SPEC_VERSION.to_string(),
            records_total: records.len(),
        },
        records,
        chain,
        final_chain_hash,
    })
}

pub(crate) fn verify_records(
    records: Vec<ScedResourceOfferRecord>,
    expected_final_chain_hash: Option<&str>,
    expected_records_total: Option<usize>,
) -> VerifierReport {
    let records_total = records.len();

    if records_total == 0 {
        let mut errors = Vec::new();
        let mut status = "PASS".to_string();
        if let Some(expected_count) = expected_records_total {
            if expected_count != 0 {
                status = "FAIL".to_string();
                errors.push(VerifyError {
                    code: VerifyCode::RecordCountMismatch,
                    message: format!("expected records_total={expected_count}, got 0"),
                    record_key: unknown_key(),
                });
            }
        }
        if let Some(expected_hash) = expected_final_chain_hash {
            if expected_hash != "0" {
                status = "FAIL".to_string();
                errors.push(VerifyError {
                    code: VerifyCode::HashMismatch,
                    message: format!("expected final_chain_hash={expected_hash}, got 0"),
                    record_key: unknown_key(),
                });
            }
        }

        return VerifierReport {
            status,
            records_total: 0,
            records_verified: 0,
            final_chain_hash: "0".to_string(),
            expected_final_chain_hash: expected_final_chain_hash.map(|s| s.to_string()),
            mismatch_index: None,
            errors,
        };
    }

    let chain = match build_hash_chain(records) {
        Ok(v) => v,
        Err(err) => {
            return VerifierReport {
                status: "FAIL".to_string(),
                records_total,
                records_verified: 0,
                final_chain_hash: String::new(),
                expected_final_chain_hash: expected_final_chain_hash.map(|s| s.to_string()),
                mismatch_index: None,
                errors: vec![map_parse_error(err)],
            };
        }
    };

    let mut errors = Vec::new();
    let mut mismatch_index = None;

    if let Some(expected_count) = expected_records_total {
        if expected_count != chain.len() {
            errors.push(VerifyError {
                code: VerifyCode::RecordCountMismatch,
                message: format!("expected records_total={expected_count}, got {}", chain.len()),
                record_key: unknown_key(),
            });
        }
    }

    let mut prev = "0".to_string();
    for (idx, item) in chain.iter().enumerate() {
        let expected_link = hash::chain_hash_hex(&prev, &item.record_hash);
        if expected_link != item.chain_hash {
            mismatch_index = Some(idx);
            errors.push(VerifyError {
                code: VerifyCode::ChainContinuityBreak,
                message: "chain continuity broken".to_string(),
                record_key: RecordKey {
                    scd_timestamp: item.key.0.clone(),
                    repeat_hour_flag: item.key.1,
                    resource_name: item.key.2.clone(),
                    offer_type: item.key.3.clone(),
                },
            });
            break;
        }
        prev = item.chain_hash.clone();
    }

    let final_chain_hash = chain
        .last()
        .map(|r| r.chain_hash.clone())
        .unwrap_or_else(|| "0".to_string());

    if mismatch_index.is_none() {
        if let Some(expected_hash) = expected_final_chain_hash {
            if expected_hash != final_chain_hash {
                mismatch_index = Some(chain.len().saturating_sub(1));
                errors.push(VerifyError {
                    code: VerifyCode::HashMismatch,
                    message: format!(
                        "expected final_chain_hash={expected_hash}, got {final_chain_hash}"
                    ),
                    record_key: chain
                        .last()
                        .map(|last| RecordKey {
                            scd_timestamp: last.key.0.clone(),
                            repeat_hour_flag: last.key.1,
                            resource_name: last.key.2.clone(),
                            offer_type: last.key.3.clone(),
                        })
                        .unwrap_or_else(unknown_key),
                });
            }
        }
    }

    VerifierReport {
        status: if errors.is_empty() {
            "PASS".to_string()
        } else {
            "FAIL".to_string()
        },
        records_total,
        records_verified: if errors.is_empty() { chain.len() } else { 0 },
        final_chain_hash,
        expected_final_chain_hash: expected_final_chain_hash.map(|s| s.to_string()),
        mismatch_index,
        errors,
    }
}

pub fn verify_truth_package(
    truth: &CanonicalTruthPackage,
    expected_final_chain_hash: Option<&str>,
) -> VerifierReport {
    let mut errors = Vec::new();
    let mut mismatch_index = None;
    let payload_records = match parse_canonical_payload_bytes(&truth.canonical_payload_bytes) {
        Ok(v) => v,
        Err(msg) => {
            return VerifierReport {
                status: "FAIL".to_string(),
                records_total: truth.metadata.records_total,
                records_verified: 0,
                final_chain_hash: truth.final_chain_hash.clone(),
                expected_final_chain_hash: expected_final_chain_hash.map(|s| s.to_string()),
                mismatch_index: None,
                errors: vec![VerifyError {
                    code: VerifyCode::CsvMalformed,
                    message: format!("canonical payload parse error: {msg}"),
                    record_key: unknown_key(),
                }],
            };
        }
    };

    if truth.metadata.records_total != payload_records.len()
        || payload_records.len() != truth.chain.len()
    {
        errors.push(VerifyError {
            code: VerifyCode::RecordCountMismatch,
            message: format!(
                "records_total={} serialized_records={} chain={}",
                truth.metadata.records_total,
                payload_records.len(),
                truth.chain.len()
            ),
            record_key: unknown_key(),
        });
    }

    let mut prev_chain_hash = "0".to_string();
    for (idx, (serialized_record, chain_item)) in payload_records.iter().zip(truth.chain.iter()).enumerate() {
        let expected_record_hash = hash::sha256_hex(serialized_record);
        if expected_record_hash != chain_item.record_hash {
            mismatch_index = Some(idx);
            errors.push(VerifyError {
                code: VerifyCode::HashMismatch,
                message: "record_hash mismatch from canonical_payload_bytes".to_string(),
                record_key: RecordKey {
                    scd_timestamp: chain_item.key.0.clone(),
                    repeat_hour_flag: chain_item.key.1,
                    resource_name: chain_item.key.2.clone(),
                    offer_type: chain_item.key.3.clone(),
                },
            });
            break;
        }

        let expected_chain_hash = sha256_hex(&format!("{}|{}", prev_chain_hash, chain_item.record_hash));
        if expected_chain_hash != chain_item.chain_hash {
            mismatch_index = Some(idx);
            errors.push(VerifyError {
                code: VerifyCode::ChainContinuityBreak,
                message: "chain continuity broken".to_string(),
                record_key: RecordKey {
                    scd_timestamp: chain_item.key.0.clone(),
                    repeat_hour_flag: chain_item.key.1,
                    resource_name: chain_item.key.2.clone(),
                    offer_type: chain_item.key.3.clone(),
                },
            });
            break;
        }
        prev_chain_hash = chain_item.chain_hash.clone();
    }

    let computed_final_chain_hash = truth
        .chain
        .last()
        .map(|r| r.chain_hash.clone())
        .unwrap_or_else(|| "0".to_string());

    if mismatch_index.is_none() && computed_final_chain_hash != truth.final_chain_hash {
        mismatch_index = Some(truth.chain.len().saturating_sub(1));
        errors.push(VerifyError {
            code: VerifyCode::HashMismatch,
            message: format!(
                "truth package final_chain_hash={} computed={}",
                truth.final_chain_hash, computed_final_chain_hash
            ),
            record_key: truth
                .records
                .last()
                .map(|r| RecordKey {
                    scd_timestamp: r.scd_timestamp.clone(),
                    repeat_hour_flag: r.repeat_hour_flag,
                    resource_name: r.resource_name.clone(),
                    offer_type: r.offer_type.clone(),
                })
                .unwrap_or_else(unknown_key),
        });
    }

    if mismatch_index.is_none() {
        if let Some(expected_hash) = expected_final_chain_hash {
            if expected_hash != computed_final_chain_hash {
                mismatch_index = Some(truth.chain.len().saturating_sub(1));
                errors.push(VerifyError {
                    code: VerifyCode::HashMismatch,
                    message: format!(
                        "expected final_chain_hash={expected_hash}, got {computed_final_chain_hash}"
                    ),
                    record_key: truth
                        .chain
                        .last()
                        .map(|r| RecordKey {
                            scd_timestamp: r.key.0.clone(),
                            repeat_hour_flag: r.key.1,
                            resource_name: r.key.2.clone(),
                            offer_type: r.key.3.clone(),
                        })
                        .unwrap_or_else(unknown_key),
                });
            }
        }
    }

    VerifierReport {
        status: if errors.is_empty() {
            "PASS".to_string()
        } else {
            "FAIL".to_string()
        },
        records_total: truth.metadata.records_total,
        records_verified: if errors.is_empty() {
            payload_records.len()
        } else {
            0
        },
        final_chain_hash: computed_final_chain_hash,
        expected_final_chain_hash: expected_final_chain_hash.map(|s| s.to_string()),
        mismatch_index,
        errors,
    }
}

fn build_canonical_payload_bytes(records: &[ScedResourceOfferRecord]) -> Vec<u8> {
    let mut out = Vec::new();
    for r in records {
        let serialized = r.canonical_record_string();
        let bytes = serialized.as_bytes();
        let len = bytes.len() as u32;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(bytes);
    }
    out
}

fn parse_canonical_payload_bytes(payload: &[u8]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 4 <= payload.len() {
        let len = u32::from_le_bytes([payload[i], payload[i + 1], payload[i + 2], payload[i + 3]]) as usize;
        i += 4;
        if i + len > payload.len() {
            return Err("declared record length exceeds payload size".to_string());
        }
        let record_bytes = &payload[i..i + len];
        i += len;
        let s = String::from_utf8(record_bytes.to_vec())
            .map_err(|_| "record payload is not valid UTF-8".to_string())?;
        out.push(s);
    }
    if i != payload.len() {
        return Err("trailing bytes after canonical payload records".to_string());
    }
    Ok(out)
}

fn map_parse_error(err: ParseError) -> VerifyError {
    match err {
        ParseError::CsvSchemaMismatch => VerifyError {
            code: VerifyCode::CsvSchemaMismatch,
            message: "CSV schema mismatch".to_string(),
            record_key: unknown_key(),
        },
        ParseError::MissingValue(field) => VerifyError {
            code: VerifyCode::CsvMalformed,
            message: format!("missing value for field '{field}'"),
            record_key: unknown_key(),
        },
        ParseError::InvalidBoolean(v) => VerifyError {
            code: VerifyCode::InvalidBoolean,
            message: format!("invalid boolean '{v}'"),
            record_key: unknown_key(),
        },
        ParseError::InvalidNumeric(field, v) => VerifyError {
            code: VerifyCode::InvalidNumeric,
            message: format!("invalid numeric field '{field}' with value '{v}'"),
            record_key: unknown_key(),
        },
        ParseError::DuplicatePrimaryKey(ts, rep, res, typ) => VerifyError {
            code: VerifyCode::DuplicatePk,
            message: "duplicate primary key detected".to_string(),
            record_key: RecordKey {
                scd_timestamp: ts,
                repeat_hour_flag: rep,
                resource_name: res,
                offer_type: typ,
            },
        },
        ParseError::MalformedCsv(msg) => VerifyError {
            code: VerifyCode::CsvMalformed,
            message: msg,
            record_key: unknown_key(),
        },
    }
}

fn unknown_key() -> RecordKey {
    RecordKey {
        scd_timestamp: String::new(),
        repeat_hour_flag: false,
        resource_name: String::new(),
        offer_type: String::new(),
    }
}

fn sha256_hex(input: &str) -> String {
    hash::sha256_hex(input)
}

fn serialize_prices_and_quantities<S>(
    values: &[String; 48],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut seq = serializer.serialize_seq(Some(48))?;
    for value in values {
        seq.serialize_element(value)?;
    }
    seq.end()
}

fn deserialize_prices_and_quantities<'de, D>(
    deserializer: D,
) -> Result<[String; 48], D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Fixed48Visitor;

    impl<'de> Visitor<'de> for Fixed48Visitor {
        type Value = [String; 48];

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an array with exactly 48 string fields")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values: Vec<String> = Vec::with_capacity(48);
            while let Some(value) = seq.next_element::<String>()? {
                values.push(value);
            }
            if values.len() != 48 {
                return Err(de::Error::invalid_length(values.len(), &self));
            }
            values
                .try_into()
                .map_err(|_| de::Error::custom("failed to convert to 48-element array"))
        }
    }

    deserializer.deserialize_seq(Fixed48Visitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_record(
        ts: &str,
        repeat: bool,
        resource: &str,
        offer: &str,
        first: &str,
    ) -> ScedResourceOfferRecord {
        let mut fields = std::array::from_fn::<_, 48, _>(|_| "0.000000".to_string());
        fields[0] = first.to_string();
        ScedResourceOfferRecord {
            scd_timestamp: ts.to_string(),
            repeat_hour_flag: repeat,
            resource_name: resource.to_string(),
            offer_type: offer.to_string(),
            prices_and_quantities: fields,
        }
    }

    #[test]
    fn canonical_order_is_fixed() {
        let r = mk_record(
            "2026-01-21T23:55:18",
            false,
            "7RNCHSLR_UNIT1",
            "OFFNS",
            "186.140000",
        );
        let serialized = r.canonical_record_string();
        assert!(serialized.starts_with("2026-01-21T23:55:18|false|7RNCHSLR_UNIT1|186.140000|"));
        assert!(serialized.ends_with("|OFFNS"));
    }

    #[test]
    fn schema_order_mismatch_fails() {
        let csv = "repeat_hour_flag,scd_timestamp,resource_name,offer_type\nfalse,2026-01-21T23:55:18,RES,OFFNS\n";
        let err = parse_csv(csv.as_bytes()).expect_err("must fail schema mismatch");
        assert_eq!(err, ParseError::CsvSchemaMismatch);
    }

    #[test]
    fn bool_is_strict_lowercase_only() {
        let csv = format!(
            "{}\n{}\n",
            canonicalize::expected_headers().join(","),
            [
                "2026-01-21T23:55:18",
                "TRUE",
                "RES",
                "OFFNS",
                &vec!["0"; 48].join(",")
            ]
            .join(",")
        );
        let err = parse_csv(csv.as_bytes()).expect_err("must reject TRUE");
        assert_eq!(err, ParseError::InvalidBoolean("TRUE".to_string()));
    }

    #[test]
    fn numeric_normalization_is_canonical_fixed_6() {
        let header = canonicalize::expected_headers().join(",");
        let mut vals = vec![
            "2026-01-21T23:55:18".to_string(),
            "false".to_string(),
            "RES".to_string(),
            "OFFNS".to_string(),
        ];
        vals.push("1".to_string());
        vals.push("1.0".to_string());
        vals.extend(vec!["0".to_string(); 46]);
        let csv = format!("{}\n{}\n", header, vals.join(","));

        let parsed = parse_csv(csv.as_bytes()).expect("parse ok");
        assert_eq!(parsed[0].prices_and_quantities[0], "1.000000");
        assert_eq!(parsed[0].prices_and_quantities[1], "1.000000");
    }

    #[test]
    fn dst_fallback_uniqueness_uses_repeat_hour_flag() {
        let a = mk_record("2026-11-01T01:30:00", false, "RES_A", "OFFNS", "10.000000");
        let b = mk_record("2026-11-01T01:30:00", true, "RES_A", "OFFNS", "10.000000");
        let chain = build_hash_chain(vec![b, a]).expect("chain should build");
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].key.1, false);
        assert_eq!(chain[1].key.1, true);
    }

    #[test]
    fn empty_file_behavior_passes_with_genesis_hash() {
        let report = verify_records(vec![], Some("0"), Some(0));
        assert_eq!(report.status, "PASS");
        assert_eq!(report.final_chain_hash, "0");
        assert_eq!(report.records_verified, 0);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn record_count_binding_enforced() {
        let r = mk_record("2026-01-21T23:55:18", false, "RES", "OFFNS", "1.000000");
        let report = verify_records(vec![r], None, Some(2));
        assert_eq!(report.status, "FAIL");
        assert_eq!(report.errors[0].code, VerifyCode::RecordCountMismatch);
    }

    #[test]
    fn mismatch_index_is_zero_based_after_sort() {
        let r = mk_record("2026-01-21T23:55:18", false, "RES", "OFFNS", "1.000000");
        let report = verify_records(vec![r], Some("not-a-real-hash"), Some(1));
        assert_eq!(report.status, "FAIL");
        assert_eq!(report.mismatch_index, Some(0));
        assert_eq!(report.errors[0].code, VerifyCode::HashMismatch);
    }

    #[test]
    fn verifier_uses_serialized_bytes_not_struct_fields() {
        let r = mk_record("2026-01-21T23:55:18", false, "RES", "OFFNS", "1.000000");
        let mut truth = compute_canonical_truth(
            format!(
                "{header}\n{row}\n",
                header = canonicalize::expected_headers().join(","),
                row = [
                    "2026-01-21T23:55:18",
                    "false",
                    "RES",
                    "OFFNS",
                    &vec!["1"; 48].join(",")
                ]
                .join(",")
            )
            .as_bytes(),
        )
        .expect("truth package should build");
        assert_eq!(truth.records.len(), 1);
        let _ = r;

        truth.canonical_payload_bytes[4] ^= 0x01;
        let report = verify_truth_package(&truth, None);
        assert_eq!(report.status, "FAIL");
        assert_eq!(report.errors[0].code, VerifyCode::HashMismatch);
    }

    #[test]
    fn struct_tamper_after_serialization_does_not_break_verification() {
        let mut truth = compute_canonical_truth(
            format!(
                "{header}\n{row}\n",
                header = canonicalize::expected_headers().join(","),
                row = [
                    "2026-01-21T23:55:18",
                    "false",
                    "RES",
                    "OFFNS",
                    &vec!["1"; 48].join(",")
                ]
                .join(",")
            )
            .as_bytes(),
        )
        .expect("truth package should build");
        truth.records[0].resource_name = "TAMPERED_STRUCT_ONLY".to_string();
        let report = verify_truth_package(&truth, Some(&truth.final_chain_hash));
        assert_eq!(report.status, "PASS");
    }
}

