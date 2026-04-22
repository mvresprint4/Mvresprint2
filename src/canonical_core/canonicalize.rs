#![deny(unsafe_code)]

use crate::canonical_core::hash::{chain_hash_hex, record_hash_hex};
use crate::canonical_core::sort::sort_records;
use crate::sced_offer_chain::{ChainedRecord, ParseError, ScedResourceOfferRecord};
use std::collections::HashSet;
use std::io::Read;

pub(crate) fn numeric_field_names() -> Vec<String> {
    let mut names = Vec::with_capacity(48);
    for block in 1..=6 {
        names.push(format!("price{block}_urs"));
        names.push(format!("price{block}_drs"));
        names.push(format!("price{block}_rrspf"));
        names.push(format!("price{block}_rrsuf"));
        names.push(format!("price{block}_rrsff"));
        names.push(format!("price{block}_ns"));
        names.push(format!("price{block}_ecrs"));
        names.push(format!("quantity_mw{block}"));
    }
    names
}

pub(crate) fn expected_headers() -> Vec<String> {
    let mut headers = vec![
        "scd_timestamp".to_string(),
        "repeat_hour_flag".to_string(),
        "resource_name".to_string(),
        "offer_type".to_string(),
    ];
    headers.extend(numeric_field_names());
    headers
}

pub(crate) fn parse_csv<R: Read>(input: R) -> Result<Vec<ScedResourceOfferRecord>, ParseError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::None)
        .from_reader(input);

    let headers = reader
        .headers()
        .map_err(|e| ParseError::MalformedCsv(e.to_string()))?;
    let found_headers: Vec<String> = headers.iter().map(|h| h.trim().to_string()).collect();
    if found_headers != expected_headers() {
        return Err(ParseError::CsvSchemaMismatch);
    }

    let mut records = Vec::new();
    for row in reader.records() {
        let row = row.map_err(|e| ParseError::MalformedCsv(e.to_string()))?;
        let raw_values: Vec<String> = row.iter().map(|v| v.trim().to_string()).collect();
        if raw_values.len() != found_headers.len() {
            return Err(ParseError::MalformedCsv(format!(
                "row has {} values but header has {}",
                raw_values.len(),
                found_headers.len()
            )));
        }
        records.push(record_from_values(&raw_values)?);
    }

    Ok(records)
}

fn record_from_values(values: &[String]) -> Result<ScedResourceOfferRecord, ParseError> {
    let scd_timestamp = required_string(values, 0, "scd_timestamp")?;
    let repeat_hour_flag = parse_bool(&required_string(values, 1, "repeat_hour_flag")?)?;
    let resource_name = required_string(values, 2, "resource_name")?;
    let offer_type = required_string(values, 3, "offer_type")?;

    let mut numeric_values = Vec::with_capacity(48);
    for (offset, key) in numeric_field_names().iter().enumerate() {
        numeric_values.push(normalize_numeric(&values[4 + offset], key)?);
    }

    let prices_and_quantities = numeric_values
        .try_into()
        .map_err(|_| ParseError::MalformedCsv("expected 48 numeric values".to_string()))?;

    Ok(ScedResourceOfferRecord {
        scd_timestamp,
        repeat_hour_flag,
        resource_name,
        offer_type,
        prices_and_quantities,
    })
}

fn required_string(values: &[String], idx: usize, key: &str) -> Result<String, ParseError> {
    let value = values
        .get(idx)
        .ok_or_else(|| ParseError::MissingValue(key.to_string()))?
        .trim()
        .to_string();
    if value.is_empty() {
        return Err(ParseError::MissingValue(key.to_string()));
    }
    Ok(value)
}

fn normalize_numeric(raw: &str, key: &str) -> Result<String, ParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return Ok("0.000000".to_string());
    }

    let parsed = trimmed
        .parse::<f64>()
        .map_err(|_| ParseError::InvalidNumeric(key.to_string(), trimmed.to_string()))?;

    Ok(format!("{parsed:.6}"))
}

fn parse_bool(raw: &str) -> Result<bool, ParseError> {
    match raw.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        "Y" | "y" => Ok(true),
        "N" | "n" => Ok(false),
        _ => Err(ParseError::InvalidBoolean(raw.to_string())),
    }
}

pub(crate) fn build_hash_chain(
    mut records: Vec<ScedResourceOfferRecord>,
) -> Result<Vec<ChainedRecord>, ParseError> {
    sort_records(&mut records);

    let mut seen = HashSet::new();
    for r in &records {
        let key = (
            r.scd_timestamp.clone(),
            r.repeat_hour_flag,
            r.resource_name.clone(),
            r.offer_type.clone(),
        );
        if !seen.insert(key.clone()) {
            return Err(ParseError::DuplicatePrimaryKey(key.0, key.1, key.2, key.3));
        }
    }

    let mut out = Vec::with_capacity(records.len());
    let mut previous_chain_hash = "0".to_string();

    for r in records {
        let record_hash = record_hash_hex(&r);
        let chain_hash = chain_hash_hex(&previous_chain_hash, &record_hash);
        previous_chain_hash = chain_hash.clone();

        out.push(ChainedRecord {
            key: (
                r.scd_timestamp,
                r.repeat_hour_flag,
                r.resource_name,
                r.offer_type,
            ),
            record_hash,
            chain_hash,
        });
    }

    Ok(out)
}
