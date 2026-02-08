use super::*;
use crate::ast::PdfDocument;

/// Basic schema implementation for core PDF validation
pub struct BasicPdfSchema {
    name: String,
    version: String,
    description: String,
}

impl BasicPdfSchema {
    pub fn new() -> Self {
        Self {
            name: "Basic PDF".to_string(),
            version: "1.0".to_string(),
            description: "Basic PDF document validation".to_string(),
        }
    }
}

impl PdfSchema for BasicPdfSchema {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn supports_pdf_version(&self, _version: &crate::ast::PdfVersion) -> bool {
        true // Basic schema supports all versions
    }

    fn validate(&self, document: &PdfDocument) -> ValidationReport {
        let mut report = ValidationReport::new(self.name().to_string(), self.version().to_string());

        // Run basic constraints
        for constraint in self.get_constraints() {
            constraint.check(document, &mut report);
        }

        report.finalize();
        report
    }

    fn get_constraints(&self) -> Vec<Box<dyn SchemaConstraint>> {
        vec![
            Box::new(BasicStructureConstraint),
            Box::new(BasicCatalogConstraint),
        ]
    }
}

impl Default for BasicPdfSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Basic structure constraint
pub struct BasicStructureConstraint;

impl SchemaConstraint for BasicStructureConstraint {
    fn name(&self) -> &str {
        "BasicStructure"
    }

    fn description(&self) -> &str {
        "Checks basic PDF document structure"
    }

    fn check(&self, document: &PdfDocument, report: &mut ValidationReport) {
        // Check if document has nodes
        if document.ast.get_all_nodes().is_empty() {
            report.add_issue(ValidationIssue {
                severity: ValidationSeverity::Critical,
                code: "EMPTY_DOCUMENT".to_string(),
                message: "Document has no nodes".to_string(),
                node_id: None,
                location: None,
                suggestion: Some("Ensure the PDF document was parsed correctly".to_string()),
            });
        } else {
            report.add_passed_check();
        }

        // Check if document has root
        if document.ast.get_root().is_none() {
            report.add_issue(ValidationIssue {
                severity: ValidationSeverity::Critical,
                code: "NO_ROOT".to_string(),
                message: "Document has no root node".to_string(),
                node_id: None,
                location: None,
                suggestion: Some("Ensure the document has a valid catalog".to_string()),
            });
        } else {
            report.add_passed_check();
        }
    }
}

/// Basic catalog constraint
pub struct BasicCatalogConstraint;

impl SchemaConstraint for BasicCatalogConstraint {
    fn name(&self) -> &str {
        "BasicCatalog"
    }

    fn description(&self) -> &str {
        "Checks basic catalog requirements"
    }

    fn check(&self, document: &PdfDocument, report: &mut ValidationReport) {
        if let Some(root_id) = document.ast.get_root() {
            if let Some(root_node) = document.ast.get_node(root_id) {
                // Check if root is catalog type
                if !matches!(root_node.node_type, crate::ast::NodeType::Catalog) {
                    report.add_issue(ValidationIssue {
                        severity: ValidationSeverity::Error,
                        code: "ROOT_NOT_CATALOG".to_string(),
                        message: "Root node is not a catalog".to_string(),
                        node_id: Some(root_id),
                        location: None,
                        suggestion: Some("Root node should be of type Catalog".to_string()),
                    });
                } else {
                    report.add_passed_check();
                }

                // Check catalog content
                if let crate::types::PdfValue::Dictionary(dict) = &root_node.value {
                    // Check Type entry
                    if let Some(type_value) = dict.get("Type") {
                        if let crate::types::PdfValue::Name(name) = type_value {
                            if name.as_str() != "/Catalog" {
                                report.add_issue(ValidationIssue {
                                    severity: ValidationSeverity::Error,
                                    code: "CATALOG_WRONG_TYPE".to_string(),
                                    message: "Catalog Type entry is not 'Catalog'".to_string(),
                                    node_id: Some(root_id),
                                    location: Some("Type".to_string()),
                                    suggestion: Some("Set Type to /Catalog".to_string()),
                                });
                            } else {
                                report.add_passed_check();
                            }
                        } else {
                            report.add_issue(ValidationIssue {
                                severity: ValidationSeverity::Error,
                                code: "CATALOG_TYPE_NOT_NAME".to_string(),
                                message: "Catalog Type entry is not a name".to_string(),
                                node_id: Some(root_id),
                                location: Some("Type".to_string()),
                                suggestion: Some("Set Type to /Catalog".to_string()),
                            });
                        }
                    } else {
                        report.add_issue(ValidationIssue {
                            severity: ValidationSeverity::Error,
                            code: "CATALOG_NO_TYPE".to_string(),
                            message: "Catalog missing Type entry".to_string(),
                            node_id: Some(root_id),
                            location: None,
                            suggestion: Some("Add Type entry with value /Catalog".to_string()),
                        });
                    }

                    // Check Pages entry
                    if !dict.contains_key("Pages") {
                        report.add_issue(ValidationIssue {
                            severity: ValidationSeverity::Error,
                            code: "CATALOG_NO_PAGES".to_string(),
                            message: "Catalog missing Pages entry".to_string(),
                            node_id: Some(root_id),
                            location: None,
                            suggestion: Some(
                                "Add Pages entry referencing the page tree".to_string(),
                            ),
                        });
                    } else {
                        report.add_passed_check();
                    }
                } else {
                    report.add_issue(ValidationIssue {
                        severity: ValidationSeverity::Critical,
                        code: "CATALOG_NOT_DICT".to_string(),
                        message: "Catalog is not a dictionary".to_string(),
                        node_id: Some(root_id),
                        location: None,
                        suggestion: Some("Catalog must be a dictionary object".to_string()),
                    });
                }
            }
        }
    }
}
