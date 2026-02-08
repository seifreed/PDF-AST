use crate::types::{PdfDictionary, PdfValue};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RichMediaInfo {
    pub assets_count: usize,
    pub configuration_count: usize,
    pub script_count: usize,
    pub asset_names: Vec<String>,
}

pub fn extract_richmedia_info(
    annotation: &PdfDictionary,
    content: Option<&PdfDictionary>,
    settings: Option<&PdfDictionary>,
) -> RichMediaInfo {
    let mut info = RichMediaInfo::default();

    if let Some(content_dict) = content {
        let (assets_count, asset_names) = extract_assets(content_dict);
        info.assets_count = assets_count;
        info.asset_names = asset_names;
        info.configuration_count = extract_configurations(content_dict);
    }

    if let Some(settings_dict) = settings {
        info.script_count = extract_scripts(settings_dict);
    } else if let Some(PdfValue::Dictionary(dict)) = annotation.get("RichMediaSettings") {
        info.script_count = extract_scripts(dict);
    }

    info
}

fn extract_assets(content: &PdfDictionary) -> (usize, Vec<String>) {
    let assets = match content.get("Assets") {
        Some(PdfValue::Dictionary(dict)) => dict,
        _ => return (0, Vec::new()),
    };

    let names = match assets.get("Names") {
        Some(PdfValue::Array(array)) => array,
        _ => return (0, Vec::new()),
    };

    let mut asset_names = Vec::new();
    let mut count = 0usize;
    let mut iter = names.iter();
    while let Some(name_val) = iter.next() {
        let name = match name_val {
            PdfValue::Name(n) => n.without_slash().to_string(),
            PdfValue::String(s) => s.decode_pdf_encoding(),
            _ => "asset".to_string(),
        };
        asset_names.push(name);
        // skip filespec or stream
        let _ = iter.next();
        count += 1;
    }

    let unique = unique_strings(asset_names);
    let count = if count == 0 { unique.len() } else { count };
    (count, unique)
}

fn extract_configurations(content: &PdfDictionary) -> usize {
    match content.get("Configurations") {
        Some(PdfValue::Array(array)) => array.len(),
        Some(PdfValue::Dictionary(_)) => 1,
        _ => 0,
    }
}

fn extract_scripts(settings: &PdfDictionary) -> usize {
    if let Some(PdfValue::Array(scripts)) = settings.get("Scripts") {
        return scripts.len();
    }

    if let Some(PdfValue::Dictionary(script)) = settings.get("Script") {
        return if script.is_empty() { 0 } else { 1 };
    }

    0
}

fn unique_strings(list: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in list {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PdfArray, PdfName, PdfString, PdfValue};

    #[test]
    fn parse_assets_and_configs() {
        let mut names = PdfArray::new();
        names.push(PdfValue::Name(PdfName::new("Video")));
        names.push(PdfValue::Dictionary(PdfDictionary::new()));
        names.push(PdfValue::String(PdfString::new_literal(b"Audio")));
        names.push(PdfValue::Dictionary(PdfDictionary::new()));

        let mut assets = PdfDictionary::new();
        assets.insert("Names", PdfValue::Array(names));

        let mut content = PdfDictionary::new();
        content.insert("Assets", PdfValue::Dictionary(assets));
        content.insert("Configurations", PdfValue::Array(PdfArray::new()));

        let info = extract_richmedia_info(&PdfDictionary::new(), Some(&content), None);
        assert_eq!(info.assets_count, 2);
        assert_eq!(info.asset_names.len(), 2);
        assert_eq!(info.configuration_count, 0);
    }

    #[test]
    fn parse_scripts() {
        let mut settings = PdfDictionary::new();
        let mut scripts = PdfArray::new();
        scripts.push(PdfValue::Dictionary(PdfDictionary::new()));
        scripts.push(PdfValue::Dictionary(PdfDictionary::new()));
        settings.insert("Scripts", PdfValue::Array(scripts));

        let info = extract_richmedia_info(&PdfDictionary::new(), None, Some(&settings));
        assert_eq!(info.script_count, 2);
    }
}
