/// Language bindings for PDF-AST
///
/// This module provides bindings for various programming languages
/// to make PDF-AST accessible from different environments.
#[cfg(feature = "python")]
pub mod python;

#[cfg(feature = "python")]
pub mod python_simple;

#[cfg(feature = "javascript")]
pub mod javascript;

/// Re-export C FFI from the ffi module
pub use crate::ffi;

/// Common utilities for language bindings
pub mod utils {
    use crate::ast::NodeType;
    use crate::types::ObjectId;

    /// Convert string to NodeType with consistent mapping
    pub fn parse_node_type(type_str: &str) -> Result<NodeType, String> {
        match type_str {
            "Root" => Ok(NodeType::Root),
            "Catalog" => Ok(NodeType::Catalog),
            "Pages" => Ok(NodeType::Pages),
            "Page" => Ok(NodeType::Page),
            "Resource" => Ok(NodeType::Resource),
            "Font" => Ok(NodeType::Font),
            "Image" => Ok(NodeType::Image),
            "ContentStream" => Ok(NodeType::ContentStream),
            "Annotation" => Ok(NodeType::Annotation),
            "Action" => Ok(NodeType::Action),
            "Metadata" => Ok(NodeType::Metadata),
            "EmbeddedFile" => Ok(NodeType::EmbeddedFile),
            "Signature" => Ok(NodeType::Signature),
            "Stream" => Ok(NodeType::Stream),
            "FilteredStream" => Ok(NodeType::FilteredStream),
            "DecodedStream" => Ok(NodeType::DecodedStream),
            "XObject" => Ok(NodeType::XObject),
            "FormXObject" => Ok(NodeType::FormXObject),
            "ImageXObject" => Ok(NodeType::ImageXObject),
            "Type1Font" => Ok(NodeType::Type1Font),
            "TrueTypeFont" => Ok(NodeType::TrueTypeFont),
            "Type3Font" => Ok(NodeType::Type3Font),
            "CIDFont" => Ok(NodeType::CIDFont),
            "JavaScriptAction" => Ok(NodeType::JavaScriptAction),
            "GoToAction" => Ok(NodeType::GoToAction),
            "URIAction" => Ok(NodeType::URIAction),
            "LaunchAction" => Ok(NodeType::LaunchAction),
            "SubmitFormAction" => Ok(NodeType::SubmitFormAction),
            "AcroForm" => Ok(NodeType::AcroForm),
            "Field" => Ok(NodeType::Field),
            "Encrypt" => Ok(NodeType::Encrypt),
            "Permission" => Ok(NodeType::Permission),
            "ContentOperator" => Ok(NodeType::ContentOperator),
            "TextOperator" => Ok(NodeType::TextOperator),
            "GraphicsOperator" => Ok(NodeType::GraphicsOperator),
            "EmbeddedJS" => Ok(NodeType::EmbeddedJS),
            "SuspiciousAction" => Ok(NodeType::SuspiciousAction),
            "ExternalReference" => Ok(NodeType::ExternalReference),
            "EncodedContent" => Ok(NodeType::EncodedContent),
            "Unknown" => Ok(NodeType::Unknown),
            // Map legacy names to existing types for compatibility
            "Outline" => Ok(NodeType::Annotation), // Outlines are a type of annotation
            "Encryption" => Ok(NodeType::Encrypt),
            "Structure" => Ok(NodeType::Resource), // Structure elements are resources
            "Form" => Ok(NodeType::AcroForm),
            "JavaScript" => Ok(NodeType::EmbeddedJS),
            "Multimedia" => Ok(NodeType::XObject), // Multimedia is typically an XObject
            "ColorSpace" => Ok(NodeType::Resource), // ColorSpace is a resource
            "Pattern" => Ok(NodeType::Resource),   // Pattern is a resource
            "Shading" => Ok(NodeType::Resource),   // Shading is a resource
            "Other" => Ok(NodeType::Unknown),
            _ => {
                // Try to parse as Object(id)
                if type_str.starts_with("Object(") && type_str.ends_with(")") {
                    let inner = &type_str[7..type_str.len() - 1];
                    let parts: Vec<&str> = inner.split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(num), Ok(gen)) = (
                            parts[0].trim().parse::<u32>(),
                            parts[1].trim().parse::<u16>(),
                        ) {
                            return Ok(NodeType::Object(ObjectId::new(num, gen)));
                        }
                    }
                }
                Err(format!("Unknown node type: {}", type_str))
            }
        }
    }

    /// Convert NodeType to string with consistent mapping
    pub fn node_type_to_string(node_type: &NodeType) -> String {
        match node_type {
            NodeType::Root => "Root".to_string(),
            NodeType::Catalog => "Catalog".to_string(),
            NodeType::Pages => "Pages".to_string(),
            NodeType::Page => "Page".to_string(),
            NodeType::Resource => "Resource".to_string(),
            NodeType::Font => "Font".to_string(),
            NodeType::Image => "Image".to_string(),
            NodeType::ContentStream => "ContentStream".to_string(),
            NodeType::Annotation => "Annotation".to_string(),
            NodeType::Action => "Action".to_string(),
            NodeType::Metadata => "Metadata".to_string(),
            NodeType::EmbeddedFile => "EmbeddedFile".to_string(),
            NodeType::Signature => "Signature".to_string(),
            NodeType::Object(obj_id) => format!("Object({}, {})", obj_id.number, obj_id.generation),
            NodeType::Unknown => "Unknown".to_string(),
            NodeType::Stream => "Stream".to_string(),
            NodeType::FilteredStream => "FilteredStream".to_string(),
            NodeType::DecodedStream => "DecodedStream".to_string(),
            NodeType::XObject => "XObject".to_string(),
            NodeType::FormXObject => "FormXObject".to_string(),
            NodeType::ImageXObject => "ImageXObject".to_string(),
            NodeType::Type1Font => "Type1Font".to_string(),
            NodeType::TrueTypeFont => "TrueTypeFont".to_string(),
            NodeType::Type3Font => "Type3Font".to_string(),
            NodeType::CIDFont => "CIDFont".to_string(),
            NodeType::JavaScriptAction => "JavaScriptAction".to_string(),
            NodeType::GoToAction => "GoToAction".to_string(),
            NodeType::URIAction => "URIAction".to_string(),
            NodeType::LaunchAction => "LaunchAction".to_string(),
            NodeType::SubmitFormAction => "SubmitFormAction".to_string(),
            NodeType::AcroForm => "AcroForm".to_string(),
            NodeType::Field => "Field".to_string(),
            NodeType::Encrypt => "Encrypt".to_string(),
            NodeType::Permission => "Permission".to_string(),
            NodeType::ContentOperator => "ContentOperator".to_string(),
            NodeType::TextOperator => "TextOperator".to_string(),
            NodeType::GraphicsOperator => "GraphicsOperator".to_string(),
            NodeType::EmbeddedJS => "EmbeddedJS".to_string(),
            NodeType::SuspiciousAction => "SuspiciousAction".to_string(),
            NodeType::ExternalReference => "ExternalReference".to_string(),
            NodeType::EncodedContent => "EncodedContent".to_string(),
            // Handle all remaining variants generically
            _ => format!("{:?}", node_type),
        }
    }

    /// List all available node types for documentation
    pub fn list_node_types() -> Vec<&'static str> {
        vec![
            "Root",
            "Catalog",
            "Pages",
            "Page",
            "Resource",
            "Font",
            "Image",
            "ContentStream",
            "Annotation",
            "Action",
            "Metadata",
            "EmbeddedFile",
            "Signature",
            "Unknown",
            "Stream",
            "FilteredStream",
            "DecodedStream",
            "XObject",
            "FormXObject",
            "ImageXObject",
            "Type1Font",
            "TrueTypeFont",
            "Type3Font",
            "CIDFont",
            "JavaScriptAction",
            "GoToAction",
            "URIAction",
            "LaunchAction",
            "SubmitFormAction",
            "AcroForm",
            "Field",
            "Encrypt",
            "Permission",
            "ContentOperator",
            "TextOperator",
            "GraphicsOperator",
            "EmbeddedJS",
            "SuspiciousAction",
            "ExternalReference",
            "EncodedContent",
            // Legacy compatibility names
            "Outline",
            "Encryption",
            "Structure",
            "Form",
            "JavaScript",
            "Multimedia",
            "ColorSpace",
            "Pattern",
            "Shading",
            "Other",
        ]
    }
}
