use crate::crypto::chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ParsedTimestamp {
    pub time: DateTime<Utc>,
    pub policy_oid: Option<String>,
    pub hash_algorithm: Option<String>,
    pub message_imprint: Option<Vec<u8>>,
    pub accuracy: Option<TimestampAccuracy>,
    pub tsa_certificate_der: Option<Vec<u8>>,
    pub tsa_chain_der: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TimestampAccuracy {
    pub seconds: u64,
    pub millis: Option<u32>,
    pub micros: Option<u32>,
}

pub fn parse_timestamp_token(data: &[u8]) -> Result<ParsedTimestamp, String> {
    #[cfg(feature = "crypto")]
    {
        if let Ok(mut ts) = parse_tst_info(data) {
            ts.tsa_certificate_der = None;
            ts.tsa_chain_der = Vec::new();
            return Ok(ts);
        }

        let (_, obj) = der_parser::der::parse_der(data)
            .map_err(|_| "Invalid DER in timestamp token".to_string())?;
        if let Some(tst_info) = extract_tst_info_from_cms(&obj) {
            let mut parsed = parse_tst_info(&tst_info)?;
            let chain = extract_tsa_certificates_der(data);
            parsed.tsa_certificate_der = chain.first().cloned();
            parsed.tsa_chain_der = chain;
            return Ok(parsed);
        }

        if let Ok(tst_info) = extract_tst_info_from_cms_openssl(data) {
            let mut parsed = parse_tst_info(&tst_info)?;
            let chain = extract_tsa_certificates_der(data);
            parsed.tsa_certificate_der = chain.first().cloned();
            parsed.tsa_chain_der = chain;
            return Ok(parsed);
        }
        Err("Timestamp token missing TSTInfo".to_string())
    }
    #[cfg(not(feature = "crypto"))]
    {
        let _ = data;
        Err("Timestamp parsing requires crypto feature".to_string())
    }
}

#[cfg(feature = "crypto")]
fn parse_tst_info(data: &[u8]) -> Result<ParsedTimestamp, String> {
    use der_parser::der::parse_der;

    let (_, obj) = parse_der(data).map_err(|_| "Invalid TSTInfo DER".to_string())?;
    let seq = obj
        .as_sequence()
        .map_err(|_| "TSTInfo not a sequence".to_string())?;

    let policy_oid = seq.get(1).and_then(|item| extract_oid_string(item));

    let (hash_algorithm, message_imprint) = seq
        .get(2)
        .and_then(|item| parse_message_imprint(item))
        .unwrap_or((None, None));

    let time = find_time_in_ber(&obj)
        .and_then(asn1_datetime_to_utc)
        .ok_or_else(|| "TSTInfo missing genTime".to_string())?;

    let accuracy = seq.iter().find_map(|item| parse_accuracy(item));

    Ok(ParsedTimestamp {
        time,
        policy_oid,
        hash_algorithm,
        message_imprint,
        accuracy,
        tsa_certificate_der: None,
        tsa_chain_der: Vec::new(),
    })
}

#[cfg(feature = "crypto")]
pub fn verify_timestamp_signature(data: &[u8]) -> Result<Vec<u8>, String> {
    use openssl::cms::{CMSOptions, CmsContentInfo};

    let mut cms =
        CmsContentInfo::from_der(data).map_err(|_| "Invalid CMS in timestamp token".to_string())?;
    let mut output = Vec::new();
    let flags = CMSOptions::BINARY | CMSOptions::NOVERIFY;
    cms.verify(None, None, None, Some(&mut output), flags)
        .map_err(|_| "Timestamp signature verification failed".to_string())?;
    Ok(output)
}

#[cfg(feature = "crypto")]
fn parse_message_imprint(
    obj: &der_parser::ber::BerObject,
) -> Option<(Option<String>, Option<Vec<u8>>)> {
    let seq = obj.as_sequence().ok()?;
    let alg_seq = seq.first()?.as_sequence().ok()?;
    let alg_oid = extract_oid_string(alg_seq.first()?);
    let alg_name = alg_oid
        .as_deref()
        .and_then(map_hash_oid)
        .map(|s| s.to_string())
        .or(alg_oid);
    let hashed = extract_octet_string(seq.get(1)?);
    Some((alg_name, hashed))
}

#[cfg(feature = "crypto")]
fn parse_accuracy(obj: &der_parser::ber::BerObject) -> Option<TimestampAccuracy> {
    let seq = obj.as_sequence().ok()?;
    let seconds = seq.first()?.as_u64().ok()?;
    let mut millis = None;
    let mut micros = None;
    for (idx, item) in seq.iter().enumerate().skip(1) {
        match idx {
            1 => millis = item.as_u64().ok().map(|v| v as u32),
            2 => micros = item.as_u64().ok().map(|v| v as u32),
            _ => {}
        }
    }
    Some(TimestampAccuracy {
        seconds,
        millis,
        micros,
    })
}

#[cfg(feature = "crypto")]
fn extract_tst_info_from_cms(obj: &der_parser::ber::BerObject) -> Option<Vec<u8>> {
    let seq = obj.as_sequence().ok()?;
    let content_type = seq.first().and_then(extract_oid_string)?;
    if content_type != "1.2.840.113549.1.7.2" {
        return None;
    }

    let signed_data = seq
        .get(1)
        .and_then(extract_explicit)
        .and_then(|inner| inner.as_sequence().ok())?;

    let encap = signed_data.get(2)?;
    let encap_seq = encap.as_sequence().ok()?;
    let econtent_type = encap_seq.first().and_then(extract_oid_string)?;
    if econtent_type != "1.2.840.113549.1.9.16.1.4" {
        return None;
    }

    encap_seq
        .get(1)
        .and_then(extract_explicit)
        .and_then(extract_octet_string)
}

#[cfg(feature = "crypto")]
fn extract_tst_info_from_cms_openssl(data: &[u8]) -> Result<Vec<u8>, String> {
    use openssl::cms::{CMSOptions, CmsContentInfo};

    let mut cms =
        CmsContentInfo::from_der(data).map_err(|_| "Invalid CMS in timestamp token".to_string())?;
    let mut output = Vec::new();
    let flags = CMSOptions::BINARY | CMSOptions::NOVERIFY;
    cms.verify(None, None, None, Some(&mut output), flags)
        .map_err(|_| "CMS content extraction failed".to_string())?;
    Ok(output)
}

#[cfg(feature = "crypto")]
pub fn extract_tsa_certificates_der(data: &[u8]) -> Vec<Vec<u8>> {
    use cms::cert::x509::der::{Decode, Encode};
    use cms::cert::CertificateChoices;
    use cms::content_info::ContentInfo;
    use cms::signed_data::SignedData;

    let content = match ContentInfo::from_der(data) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    if content.content_type.to_string() != "1.2.840.113549.1.7.2" {
        return Vec::new();
    }

    let signed_der = match content.content.to_der() {
        Ok(der) => der,
        Err(_) => return Vec::new(),
    };
    let signed = match SignedData::from_der(&signed_der) {
        Ok(sd) => sd,
        Err(_) => return Vec::new(),
    };
    let certs = match signed.certificates {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut out = Vec::new();
    for cert in certs.0.iter() {
        if let CertificateChoices::Certificate(c) = cert {
            if let Ok(der) = c.to_der() {
                out.push(der);
            }
        }
    }
    out
}

#[cfg(feature = "crypto")]
fn extract_octet_string(obj: &der_parser::ber::BerObject) -> Option<Vec<u8>> {
    use der_parser::ber::BerObjectContent;
    match &obj.content {
        BerObjectContent::OctetString(data) => Some(data.to_vec()),
        _ => None,
    }
}

#[cfg(feature = "crypto")]
fn extract_oid_string(obj: &der_parser::ber::BerObject) -> Option<String> {
    use der_parser::ber::BerObjectContent;
    match &obj.content {
        BerObjectContent::OID(oid) => Some(oid.to_string()),
        _ => None,
    }
}

#[cfg(feature = "crypto")]
fn extract_explicit<'a>(
    obj: &'a der_parser::ber::BerObject<'a>,
) -> Option<&'a der_parser::ber::BerObject<'a>> {
    use der_parser::ber::BerObjectContent;
    match &obj.content {
        BerObjectContent::Tagged(_, _, inner) => Some(inner),
        _ => None,
    }
}

#[cfg(feature = "crypto")]
fn map_hash_oid(oid: &str) -> Option<&'static str> {
    match oid {
        "1.3.14.3.2.26" => Some("SHA-1"),
        "2.16.840.1.101.3.4.2.1" => Some("SHA-256"),
        "2.16.840.1.101.3.4.2.2" => Some("SHA-384"),
        "2.16.840.1.101.3.4.2.3" => Some("SHA-512"),
        _ => None,
    }
}

#[cfg(feature = "crypto")]
fn find_time_in_ber(obj: &der_parser::ber::BerObject) -> Option<der_parser::asn1_rs::ASN1DateTime> {
    use der_parser::ber::BerObjectContent;
    match &obj.content {
        BerObjectContent::UTCTime(t) | BerObjectContent::GeneralizedTime(t) => Some(t.clone()),
        BerObjectContent::Sequence(items) | BerObjectContent::Set(items) => {
            for item in items {
                if let Some(time) = find_time_in_ber(item) {
                    return Some(time);
                }
            }
            None
        }
        BerObjectContent::Optional(Some(inner)) => find_time_in_ber(inner),
        BerObjectContent::Tagged(_, _, inner) => find_time_in_ber(inner),
        _ => None,
    }
}

#[cfg(feature = "crypto")]
fn asn1_datetime_to_utc(time: der_parser::asn1_rs::ASN1DateTime) -> Option<DateTime<Utc>> {
    use der_parser::asn1_rs::ASN1TimeZone;
    let date =
        ::chrono::NaiveDate::from_ymd_opt(time.year as i32, time.month as u32, time.day as u32)?;
    let dt = date.and_hms_opt(time.hour as u32, time.minute as u32, time.second as u32)?;
    let mut ts = dt.and_utc().timestamp();

    match time.tz {
        ASN1TimeZone::Offset(h, m) => {
            let offset = (h as i64) * 3600 + (m as i64) * 60;
            ts -= offset;
        }
        ASN1TimeZone::Z | ASN1TimeZone::Undefined => {}
    }

    Some(DateTime::from_timestamp(ts))
}
