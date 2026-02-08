use crate::ast::PdfDocument;
use crate::security::{ValidationResult, ValidationStatus};

pub fn detect_producer_quirks(document: &PdfDocument) -> Vec<ValidationResult> {
    let mut results = Vec::new();
    let producer = document
        .metadata
        .producer
        .clone()
        .unwrap_or_else(|| "".to_string());
    let creator = document
        .metadata
        .creator
        .clone()
        .unwrap_or_else(|| "".to_string());

    let combined = format!("{} {}", producer, creator).to_lowercase();

    if combined.contains("adobe") {
        results.push(ValidationResult {
            check_type: "Quirk:Adobe".to_string(),
            status: ValidationStatus::Warning,
            message: "Adobe producer/creator detected; consider Adobe-specific quirks".to_string(),
        });
    }

    if combined.contains("apple") || combined.contains("quartz") || combined.contains("mac os x") {
        results.push(ValidationResult {
            check_type: "Quirk:Apple".to_string(),
            status: ValidationStatus::Warning,
            message: "Apple/Quartz producer detected; Preview-specific quirks possible".to_string(),
        });
    }

    if combined.contains("mupdf") {
        results.push(ValidationResult {
            check_type: "Quirk:MuPDF".to_string(),
            status: ValidationStatus::Warning,
            message: "MuPDF producer detected; nonstandard object streams possible".to_string(),
        });
    }

    if combined.contains("ghostscript") {
        results.push(ValidationResult {
            check_type: "Quirk:Ghostscript".to_string(),
            status: ValidationStatus::Warning,
            message: "Ghostscript producer detected; optional entries may be omitted".to_string(),
        });
    }

    results
}
