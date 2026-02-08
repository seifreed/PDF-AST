use pdf_ast::security::output_format_from_path;
use pdf_ast::SecurityOutputFormat;
use std::path::Path;

#[test]
fn detects_report_format_from_extension() {
    assert_eq!(
        output_format_from_path(Path::new("out.yaml")),
        Some(SecurityOutputFormat::Yaml)
    );
    assert_eq!(
        output_format_from_path(Path::new("out.yml")),
        Some(SecurityOutputFormat::Yaml)
    );
    assert_eq!(
        output_format_from_path(Path::new("out.toml")),
        Some(SecurityOutputFormat::Toml)
    );
    assert_eq!(
        output_format_from_path(Path::new("out.json")),
        Some(SecurityOutputFormat::Json)
    );
    assert_eq!(output_format_from_path(Path::new("out.txt")), None);
}
