#![deny(unsafe_code)]

use crate::sced_offer_chain::ScedResourceOfferRecord;

pub(crate) fn canonical_record_string_utf8(record: &ScedResourceOfferRecord) -> String {
    let mut fields = Vec::with_capacity(52);
    fields.push(record.scd_timestamp.clone());
    fields.push(record.repeat_hour_flag.to_string());
    fields.push(record.resource_name.clone());
    fields.extend(record.prices_and_quantities.iter().cloned());
    fields.push(record.offer_type.clone());
    fields.join("|")
}
