#![deny(unsafe_code)]

use crate::sced_offer_chain::ScedResourceOfferRecord;
use std::cmp::Ordering;

pub(crate) fn sort_records(records: &mut [ScedResourceOfferRecord]) {
    records.sort_by(compare_records);
}

fn compare_records(a: &ScedResourceOfferRecord, b: &ScedResourceOfferRecord) -> Ordering {
    a.scd_timestamp
        .cmp(&b.scd_timestamp)
        .then(a.repeat_hour_flag.cmp(&b.repeat_hour_flag))
        .then(a.resource_name.cmp(&b.resource_name))
        .then(a.offer_type.cmp(&b.offer_type))
}
