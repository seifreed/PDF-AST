pub struct SignatureVerifier;

impl SignatureVerifier {
    pub fn verify_pkcs7_signature(
        contents: &[u8],
        signed_data: &[u8],
    ) -> SignatureVerificationResult {
        let handler = crate::crypto::pkcs7::Pkcs7Handler::new();
        match handler.verify_pkcs7(contents, signed_data) {
            Ok(result) => SignatureVerificationResult {
                is_valid: result.is_valid,
                error: result.error_message.clone(),
                certificate_chain: result
                    .certificate_chain
                    .iter()
                    .map(|c| CertificateInfo {
                        subject: c.subject.clone(),
                        issuer: c.issuer.clone(),
                        serial_number: c.serial_number.clone(),
                        not_before: format!("{}", c.not_before),
                        not_after: format!("{}", c.not_after),
                        public_key_algorithm: c.public_key_algorithm.clone(),
                        signature_algorithm: c.signature_algorithm.clone(),
                        key_usage: c.key_usage.clone(),
                        extended_key_usage: c.extended_key_usage.clone(),
                    })
                    .collect(),
                signing_time: result.signing_time.map(|t| format!("{}", t)),
                timestamp_info: result.timestamp_info.as_ref().map(|t| TimestampInfo {
                    timestamp_authority: t.timestamp_authority.clone(),
                    timestamp: format!("{}", t.timestamp),
                    hash_algorithm: t.hash_algorithm.clone(),
                    is_valid: t.is_valid,
                }),
            },
            Err(e) => SignatureVerificationResult {
                is_valid: false,
                error: Some(format!("PKCS#7 verification error: {}", e)),
                certificate_chain: Vec::new(),
                signing_time: None,
                timestamp_info: None,
            },
        }
    }

    pub fn verify_x509_signature(
        _contents: &[u8],
        _signed_data: &[u8],
    ) -> SignatureVerificationResult {
        SignatureVerificationResult {
            is_valid: false,
            error: Some("X.509 verification requires certificate and data context".to_string()),
            certificate_chain: Vec::new(),
            signing_time: None,
            timestamp_info: None,
        }
    }

    pub fn extract_signature_info(contents: &[u8]) -> Result<SignatureInfo, String> {
        // Parse the signature contents to extract basic information
        // This would normally parse ASN.1/DER encoded PKCS#7 data

        if contents.len() < 10 {
            return Err("Signature contents too short".to_string());
        }

        // Check for PKCS#7 magic bytes
        if contents.starts_with(&[0x30, 0x82]) || contents.starts_with(&[0x30, 0x80]) {
            Ok(SignatureInfo {
                format: SignatureFormat::PKCS7,
                algorithm: "Unknown".to_string(),
                signer_info: None,
                certificates: Vec::new(),
                timestamp: None,
            })
        } else {
            Err("Unknown signature format".to_string())
        }
    }
}

#[derive(Debug, Clone)]
pub struct SignatureVerificationResult {
    pub is_valid: bool,
    pub error: Option<String>,
    pub certificate_chain: Vec<CertificateInfo>,
    pub signing_time: Option<String>,
    pub timestamp_info: Option<TimestampInfo>,
}

#[derive(Debug, Clone)]
pub struct SignatureInfo {
    pub format: SignatureFormat,
    pub algorithm: String,
    pub signer_info: Option<SignerInfo>,
    pub certificates: Vec<CertificateInfo>,
    pub timestamp: Option<TimestampInfo>,
}

#[derive(Debug, Clone)]
pub enum SignatureFormat {
    PKCS7,
    X509,
    CAdES,
    PAdES,
}

#[derive(Debug, Clone)]
pub struct SignerInfo {
    pub issuer: String,
    pub serial_number: String,
    pub digest_algorithm: String,
    pub signature_algorithm: String,
}

#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub not_before: String,
    pub not_after: String,
    pub public_key_algorithm: String,
    pub signature_algorithm: String,
    pub key_usage: Vec<String>,
    pub extended_key_usage: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TimestampInfo {
    pub timestamp_authority: String,
    pub timestamp: String,
    pub hash_algorithm: String,
    pub is_valid: bool,
}

pub fn to_digital_signature(
    info: &crate::crypto::signature_verification::SignatureInfo,
) -> crate::security::DigitalSignature {
    let signature_type = match info.sub_filter.as_str() {
        "adbe.pkcs7.detached" => crate::security::SignatureType::AdbePkcs7Detached,
        "adbe.pkcs7.sha1" => crate::security::SignatureType::AdbePkcs7Sha1,
        "adbe.x509.rsa_sha1" => crate::security::SignatureType::AdbeX509RsaSha1,
        "ETSI.CAdES.detached" => crate::security::SignatureType::EtsiCadEsDetached,
        "ETSI.RFC3161" => crate::security::SignatureType::EtsiRfc3161,
        _ => crate::security::SignatureType::AdbePkcs7Detached,
    };

    let signer = if info.signer.subject.is_empty() {
        None
    } else {
        Some(info.signer.subject.clone())
    };

    let certificate_info = if info.signer.subject.is_empty() && info.signer.issuer.is_empty() {
        None
    } else {
        Some(crate::security::CertificateInfo {
            issuer: info.signer.issuer.clone(),
            subject: info.signer.subject.clone(),
            serial_number: bytes_to_hex(&info.signer.serial_number),
            valid_from: info.signer.not_before.to_string(),
            valid_to: info.signer.not_after.to_string(),
            key_usage: info.signer.key_usage.clone(),
            algorithm: info.filter.clone(),
        })
    };

    let validity = match &info.validity {
        crate::crypto::signature_verification::SignatureValidity::Valid => {
            crate::security::SignatureValidity::Valid
        }
        crate::crypto::signature_verification::SignatureValidity::Unknown(msg) => {
            crate::security::SignatureValidity::Unknown(msg.clone())
        }
        crate::crypto::signature_verification::SignatureValidity::Invalid(msg) => {
            crate::security::SignatureValidity::Invalid(msg.clone())
        }
        other => crate::security::SignatureValidity::Unknown(format!("{:?}", other)),
    };

    let timestamp = info
        .timestamp
        .as_ref()
        .map(|ts| crate::security::TimestampDetails {
            time: Some(ts.time.to_string()),
            policy_oid: ts.policy_oid.clone(),
            hash_algorithm: ts.hash_algorithm.clone(),
            signature_valid: ts.signature_valid,
            tsa_chain_valid: ts.tsa_chain_valid,
            tsa_pin_valid: ts.tsa_pin_valid,
            tsa_revocation_events: ts.tsa_revocation_events.clone(),
        });

    crate::security::DigitalSignature {
        field_name: info.field_name.clone(),
        signature_type,
        signer,
        signing_time: info.signing_time.map(|t| t.to_string()),
        certificate_info,
        validity,
        location: info.location.clone(),
        reason: info.reason.clone(),
        contact_info: info.contact_info.clone(),
        timestamp,
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

pub fn parse_pdf_date(date_str: &str) -> Result<String, String> {
    // Parse PDF date format: D:YYYYMMDDHHmmSSOHH'mm
    if !date_str.starts_with("D:") {
        return Err("Invalid PDF date format".to_string());
    }

    let date_part = &date_str[2..];
    if date_part.len() < 14 {
        return Err("PDF date too short".to_string());
    }

    let year = &date_part[0..4];
    let month = &date_part[4..6];
    let day = &date_part[6..8];
    let hour = &date_part[8..10];
    let minute = &date_part[10..12];
    let second = &date_part[12..14];

    Ok(format!(
        "{}-{}-{} {}:{}:{}",
        year, month, day, hour, minute, second
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pdf_date() {
        let date = "D:20231201120000+01'00";
        let result = parse_pdf_date(date);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2023-12-01 12:00:00");
    }

    #[test]
    fn test_invalid_pdf_date() {
        let date = "InvalidDate";
        let result = parse_pdf_date(date);
        assert!(result.is_err());
    }
}
