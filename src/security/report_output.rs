use crate::security::{
    security_report_to_json, security_report_to_toml, security_report_to_yaml, SecurityReport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityOutputFormat {
    Json,
    Yaml,
    Toml,
}

pub fn format_security_report(
    report: &SecurityReport,
    format: SecurityOutputFormat,
) -> Result<String, String> {
    match format {
        SecurityOutputFormat::Json => security_report_to_json(report),
        SecurityOutputFormat::Yaml => security_report_to_yaml(report),
        SecurityOutputFormat::Toml => security_report_to_toml(report),
    }
}

pub fn output_format_from_path(path: &std::path::Path) -> Option<SecurityOutputFormat> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "json" => Some(SecurityOutputFormat::Json),
        "yaml" | "yml" => Some(SecurityOutputFormat::Yaml),
        "toml" => Some(SecurityOutputFormat::Toml),
        _ => None,
    }
}
