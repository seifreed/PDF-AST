use crate::types::{PdfDictionary, PdfValue};

#[derive(Debug, Clone)]
pub struct AcroFormStats {
    pub field_count: usize,
    pub has_fields: bool,
}

impl AcroFormStats {
    pub fn new(field_count: usize) -> Self {
        Self {
            field_count,
            has_fields: field_count > 0,
        }
    }
}

pub fn count_fields_in_acroform(acroform: &PdfDictionary) -> AcroFormStats {
    let mut count = 0usize;
    if let Some(fields_value) = acroform.get("Fields") {
        count_fields_value(fields_value, &mut count, 0);
    }
    AcroFormStats::new(count)
}

fn count_fields_value(value: &PdfValue, count: &mut usize, depth: usize) {
    if depth > 64 {
        return;
    }
    match value {
        PdfValue::Array(items) => {
            for item in items.iter() {
                count_fields_value(item, count, depth + 1);
            }
        }
        PdfValue::Dictionary(dict) => {
            if is_field_dict(dict) {
                *count += 1;
            }
            if let Some(kids) = dict.get("Kids") {
                count_fields_value(kids, count, depth + 1);
            }
        }
        _ => {}
    }
}

fn is_field_dict(dict: &PdfDictionary) -> bool {
    dict.get("T").is_some() || dict.get("FT").is_some() || dict.get("Kids").is_some()
}

pub fn has_hybrid_forms(has_xfa: bool, acroform: &PdfDictionary) -> bool {
    if !has_xfa {
        return false;
    }
    let stats = count_fields_in_acroform(acroform);
    stats.has_fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PdfArray;
    use crate::types::{PdfName, PdfString};

    #[test]
    fn count_fields_simple() {
        let mut field = PdfDictionary::new();
        field.insert("T", PdfValue::String(PdfString::new_literal(b"Field1")));
        field.insert("FT", PdfValue::Name(PdfName::new("Tx")));

        let mut fields = PdfArray::new();
        fields.push(PdfValue::Dictionary(field));

        let mut acroform = PdfDictionary::new();
        acroform.insert("Fields", PdfValue::Array(fields));

        let stats = count_fields_in_acroform(&acroform);
        assert_eq!(stats.field_count, 1);
    }

    #[test]
    fn count_fields_nested_kids() {
        let mut child = PdfDictionary::new();
        child.insert("T", PdfValue::String(PdfString::new_literal(b"Child")));

        let mut kids = PdfArray::new();
        kids.push(PdfValue::Dictionary(child));

        let mut parent = PdfDictionary::new();
        parent.insert("Kids", PdfValue::Array(kids));

        let mut fields = PdfArray::new();
        fields.push(PdfValue::Dictionary(parent));

        let mut acroform = PdfDictionary::new();
        acroform.insert("Fields", PdfValue::Array(fields));

        let stats = count_fields_in_acroform(&acroform);
        assert_eq!(stats.field_count, 2);
    }

    #[test]
    fn hybrid_forms_detection() {
        let mut field = PdfDictionary::new();
        field.insert("T", PdfValue::String(PdfString::new_literal(b"Field1")));
        let mut fields = PdfArray::new();
        fields.push(PdfValue::Dictionary(field));

        let mut acroform = PdfDictionary::new();
        acroform.insert("Fields", PdfValue::Array(fields));

        assert!(has_hybrid_forms(true, &acroform));
        assert!(!has_hybrid_forms(false, &acroform));
    }
}
