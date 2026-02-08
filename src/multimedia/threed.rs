use crate::types::{PdfDictionary, PdfStream};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreeDInfo {
    pub format: Option<String>,
    pub byte_len: usize,
    pub has_view: bool,
    pub has_activation: bool,
}

pub fn extract_threed_info(annotation: &PdfDictionary, stream: Option<&PdfStream>) -> ThreeDInfo {
    let mut info = ThreeDInfo::default();

    if let Some(stream) = stream {
        info.byte_len = stream
            .raw_data()
            .map(|d| d.len())
            .unwrap_or_else(|| stream.data.len());
        info.format = detect_3d_format(stream);
    }

    if annotation.get("3DV").is_some() {
        info.has_view = true;
    }

    if annotation.get("3DA").is_some() {
        info.has_activation = true;
    }

    info
}

fn detect_3d_format(stream: &PdfStream) -> Option<String> {
    if let Some(subtype) = stream
        .dict
        .get("Subtype")
        .and_then(|v| v.as_name())
        .map(|n| n.without_slash().to_string())
    {
        if subtype.eq_ignore_ascii_case("U3D") {
            return Some("U3D".to_string());
        }
        if subtype.eq_ignore_ascii_case("PRC") {
            return Some("PRC".to_string());
        }
    }

    let data = stream.raw_data()?;

    if data.len() >= 4 && &data[0..4] == b"U3D\0" {
        return Some("U3D".to_string());
    }

    if data.len() >= 4 && &data[0..4] == b"PRC\0" {
        return Some("PRC".to_string());
    }

    None
}

pub fn aggregate_format_counts(formats: &[Option<String>]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for fmt in formats.iter().flatten() {
        *counts.entry(fmt.clone()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PdfDictionary, PdfValue};

    #[test]
    fn detect_u3d_subtype() {
        let mut dict = PdfDictionary::new();
        dict.insert("Subtype", PdfValue::Name("U3D".into()));
        let stream = PdfStream::new(dict, vec![0u8; 4]);
        let fmt = detect_3d_format(&stream);
        assert_eq!(fmt, Some("U3D".to_string()));
    }

    #[test]
    fn detect_prc_header() {
        let dict = PdfDictionary::new();
        let stream = PdfStream::new(dict, b"PRC\0xxxx".to_vec());
        let fmt = detect_3d_format(&stream);
        assert_eq!(fmt, Some("PRC".to_string()));
    }

    #[test]
    fn threed_info_flags() {
        let mut annot = PdfDictionary::new();
        annot.insert("3DV", PdfValue::Dictionary(PdfDictionary::new()));
        annot.insert("3DA", PdfValue::Dictionary(PdfDictionary::new()));
        let info = extract_threed_info(&annot, None);
        assert!(info.has_view);
        assert!(info.has_activation);
    }
}
