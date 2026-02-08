use crate::security::{
    DigitalSignature, SignatureType, SignatureValidity, ValidationResult, ValidationStatus,
};

#[derive(Debug, Clone, Default)]
pub struct EtsiValidationOptions {
    pub require_dss_for_pades: bool,
}

pub fn validate_etsi_profiles(
    signatures: &[DigitalSignature],
    has_dss: bool,
    options: EtsiValidationOptions,
) -> Vec<ValidationResult> {
    let mut results = Vec::new();

    let mut cades_found = 0usize;
    let mut pades_found = 0usize;
    let mut rfc3161_found = 0usize;

    for sig in signatures {
        match sig.signature_type {
            SignatureType::EtsiCadEsDetached => {
                cades_found += 1;
                results.push(result_for_signature(sig, "ETSI:CAdES"));
            }
            SignatureType::EtsiRfc3161 => {
                rfc3161_found += 1;
                results.push(result_for_timestamp(sig));
            }
            SignatureType::AdbePkcs7Detached
            | SignatureType::AdbePkcs7Sha1
            | SignatureType::AdbeX509RsaSha1 => {
                // Potential PAdES baseline
                pades_found += 1;
            }
        }
    }

    if pades_found > 0 {
        let status = if options.require_dss_for_pades && !has_dss {
            ValidationStatus::Fail
        } else if !has_dss {
            ValidationStatus::Warning
        } else {
            ValidationStatus::Pass
        };
        results.push(ValidationResult {
            check_type: "ETSI:PAdES-LTV".to_string(),
            status,
            message: if has_dss {
                format!(
                    "PAdES signatures detected: {} with DSS present",
                    pades_found
                )
            } else {
                format!("PAdES signatures detected: {} without DSS", pades_found)
            },
        });
    }

    if cades_found == 0 && rfc3161_found == 0 && pades_found == 0 {
        results.push(ValidationResult {
            check_type: "ETSI:Profiles".to_string(),
            status: ValidationStatus::Warning,
            message: "No ETSI/PAdES signatures detected".to_string(),
        });
    }

    results
}

fn result_for_signature(sig: &DigitalSignature, label: &str) -> ValidationResult {
    match &sig.validity {
        SignatureValidity::Valid => ValidationResult {
            check_type: label.to_string(),
            status: ValidationStatus::Pass,
            message: format!("{} signature valid", label),
        },
        SignatureValidity::Invalid(msg) => ValidationResult {
            check_type: label.to_string(),
            status: ValidationStatus::Fail,
            message: format!("{} signature invalid: {}", label, msg),
        },
        SignatureValidity::Unknown(msg) => ValidationResult {
            check_type: label.to_string(),
            status: ValidationStatus::Warning,
            message: format!("{} signature unknown: {}", label, msg),
        },
    }
}

fn result_for_timestamp(sig: &DigitalSignature) -> ValidationResult {
    if let Some(ts) = &sig.timestamp {
        let status = if ts.signature_valid {
            ValidationStatus::Pass
        } else {
            ValidationStatus::Fail
        };
        return ValidationResult {
            check_type: "ETSI:RFC3161".to_string(),
            status,
            message: if ts.signature_valid {
                "RFC3161 timestamp valid".to_string()
            } else {
                "RFC3161 timestamp invalid".to_string()
            },
        };
    }

    ValidationResult {
        check_type: "ETSI:RFC3161".to_string(),
        status: ValidationStatus::Warning,
        message: "RFC3161 timestamp not present".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{DigitalSignature, SignatureType, SignatureValidity, TimestampDetails};

    fn base_sig(signature_type: SignatureType) -> DigitalSignature {
        DigitalSignature {
            field_name: "Sig1".to_string(),
            signature_type,
            signer: None,
            signing_time: None,
            certificate_info: None,
            validity: SignatureValidity::Valid,
            location: None,
            reason: None,
            contact_info: None,
            timestamp: None,
        }
    }

    #[test]
    fn validate_pades_requires_dss() {
        let sigs = vec![base_sig(SignatureType::AdbePkcs7Detached)];
        let res = validate_etsi_profiles(
            &sigs,
            false,
            EtsiValidationOptions {
                require_dss_for_pades: true,
            },
        );
        assert!(res.iter().any(|r| r.status == ValidationStatus::Fail));
    }

    #[test]
    fn validate_cades_and_rfc3161() {
        let mut sig = base_sig(SignatureType::EtsiRfc3161);
        sig.timestamp = Some(TimestampDetails {
            time: None,
            policy_oid: None,
            hash_algorithm: None,
            signature_valid: true,
            tsa_chain_valid: None,
            tsa_pin_valid: None,
            tsa_revocation_events: Vec::new(),
        });
        let sigs = vec![sig, base_sig(SignatureType::EtsiCadEsDetached)];
        let res = validate_etsi_profiles(&sigs, true, EtsiValidationOptions::default());
        assert!(res.iter().any(|r| r.check_type == "ETSI:CAdES"));
        assert!(res.iter().any(|r| r.check_type == "ETSI:RFC3161"));
    }
}
