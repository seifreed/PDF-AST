use crate::ast::{AstNode, NodeId, NodeType, PdfDocument, PdfVersion};
use crate::parser::PdfParser;
use crate::plugins::api::PluginManager;
use crate::validation::{SchemaRegistry, ValidationReport};
use neon::prelude::*;
use neon::types::buffer::TypedArray;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// JavaScript wrapper for PdfDocument
pub struct JsPdfDocument {
    inner: Arc<Mutex<PdfDocument>>,
}

impl Finalize for JsPdfDocument {}

impl JsPdfDocument {
    fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<JsPdfDocument>> {
        let document = JsPdfDocument {
            inner: Arc::new(Mutex::new(PdfDocument::new(PdfVersion {
                major: 1,
                minor: 7,
            }))),
        };
        Ok(cx.boxed(document))
    }

    fn js_from_buffer(mut cx: FunctionContext) -> JsResult<JsBox<JsPdfDocument>> {
        let buffer = cx.argument::<JsBuffer>(0)?;
        let data = buffer.as_slice(&cx);

        let parser = PdfParser::new();
        match parser.parse_bytes(data) {
            Ok(document) => {
                let js_document = JsPdfDocument {
                    inner: Arc::new(Mutex::new(document)),
                };
                Ok(cx.boxed(js_document))
            }
            Err(e) => cx.throw_error(format!("Failed to parse PDF: {:?}", e)),
        }
    }

    fn js_get_version(mut cx: FunctionContext) -> JsResult<JsObject> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPdfDocument>, _>(&mut cx)?;
        let document = this.inner.lock().unwrap();

        let version = cx.empty_object();
        let major = cx.number(document.version.major);
        let minor = cx.number(document.version.minor);

        version.set(&mut cx, "major", major)?;
        version.set(&mut cx, "minor", minor)?;

        Ok(version)
    }

    fn js_get_all_nodes(mut cx: FunctionContext) -> JsResult<JsArray> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPdfDocument>, _>(&mut cx)?;
        let document = this.inner.lock().unwrap();

        let nodes = document.ast.get_all_nodes();
        let js_array = cx.empty_array();

        for (i, node) in nodes.iter().enumerate() {
            let js_node = JsAstNode::from_node(&mut cx, node.clone())?;
            js_array.set(&mut cx, i as u32, js_node)?;
        }

        Ok(js_array)
    }

    fn js_get_root(mut cx: FunctionContext) -> JsResult<JsValue> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPdfDocument>, _>(&mut cx)?;
        let document = this.inner.lock().unwrap();

        if let Some(root_id) = document.ast.get_root() {
            if let Some(root_node) = document.ast.get_node(root_id) {
                let js_node = JsAstNode::from_node(&mut cx, root_node.clone())?;
                return Ok(js_node.upcast());
            }
        }

        Ok(cx.null().upcast())
    }

    fn js_get_node(mut cx: FunctionContext) -> JsResult<JsValue> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPdfDocument>, _>(&mut cx)?;
        let node_id = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;
        let document = this.inner.lock().unwrap();

        if let Some(node) = document.ast.get_node(NodeId(node_id)) {
            let js_node = JsAstNode::from_node(&mut cx, node.clone())?;
            Ok(js_node.upcast())
        } else {
            Ok(cx.null().upcast())
        }
    }

    fn js_get_children(mut cx: FunctionContext) -> JsResult<JsArray> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPdfDocument>, _>(&mut cx)?;
        let node_id = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;
        let document = this.inner.lock().unwrap();

        let children = document.ast.get_children(NodeId(node_id));
        let js_array = cx.empty_array();

        for (i, &child_id) in children.iter().enumerate() {
            if let Some(child_node) = document.ast.get_node(child_id) {
                let js_node = JsAstNode::from_node(&mut cx, child_node.clone())?;
                js_array.set(&mut cx, i as u32, js_node)?;
            }
        }

        Ok(js_array)
    }

    fn js_get_nodes_by_type(mut cx: FunctionContext) -> JsResult<JsArray> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPdfDocument>, _>(&mut cx)?;
        let type_str = cx.argument::<JsString>(0)?.value(&mut cx);
        let document = this.inner.lock().unwrap();

        let js_array = cx.empty_array();

        if let Ok(node_type) = super::utils::parse_node_type(&type_str) {
            let nodes = document.ast.get_nodes_by_type(&node_type);

            for (i, node) in nodes.iter().enumerate() {
                let js_node = JsAstNode::from_node(&mut cx, node.clone())?;
                js_array.set(&mut cx, i as u32, js_node)?;
            }
        }

        Ok(js_array)
    }

    fn js_validate(mut cx: FunctionContext) -> JsResult<JsBox<JsValidationReport>> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPdfDocument>, _>(&mut cx)?;
        let schema_name = cx.argument::<JsString>(0)?.value(&mut cx);
        let document = this.inner.lock().unwrap();

        let registry = SchemaRegistry::new();

        match registry.validate(&document, &schema_name) {
            Some(report) => {
                let js_report = JsValidationReport {
                    inner: Arc::new(report),
                };
                Ok(cx.boxed(js_report))
            }
            None => cx.throw_error(format!("Schema '{}' not found", schema_name)),
        }
    }

    fn js_get_statistics(mut cx: FunctionContext) -> JsResult<JsObject> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPdfDocument>, _>(&mut cx)?;
        let document = this.inner.lock().unwrap();

        let stats = cx.empty_object();
        let total_nodes = document.ast.get_all_nodes().len();
        stats.set(&mut cx, "totalNodes", cx.number(total_nodes))?;

        // Count nodes by type
        let mut type_counts = HashMap::new();
        for node in document.ast.get_all_nodes() {
            let type_name = super::utils::node_type_to_string(&node.node_type);
            *type_counts.entry(type_name).or_insert(0) += 1;
        }

        let type_stats = cx.empty_object();
        for (node_type, count) in type_counts {
            type_stats.set(&mut cx, node_type, cx.number(count))?;
        }
        stats.set(&mut cx, "nodeTypes", type_stats)?;

        let version_str = format!("{}.{}", document.version.major, document.version.minor);
        stats.set(&mut cx, "version", cx.string(version_str))?;

        Ok(stats)
    }
}

/// JavaScript wrapper for AstNode
pub struct JsAstNode {
    inner: AstNode,
}

impl Finalize for JsAstNode {}

impl JsAstNode {
    fn from_node(cx: &mut FunctionContext, node: AstNode) -> JsResult<JsBox<JsAstNode>> {
        let js_node = JsAstNode { inner: node };
        Ok(cx.boxed(js_node))
    }

    fn js_get_id(mut cx: FunctionContext) -> JsResult<JsNumber> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsAstNode>, _>(&mut cx)?;
        Ok(cx.number(this.inner.id.0 as f64))
    }

    fn js_get_type(mut cx: FunctionContext) -> JsResult<JsString> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsAstNode>, _>(&mut cx)?;
        let type_name = super::utils::node_type_to_string(&this.inner.node_type);
        Ok(cx.string(type_name))
    }

    fn js_get_value(mut cx: FunctionContext) -> JsResult<JsString> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsAstNode>, _>(&mut cx)?;
        let value_str = format!("{:?}", this.inner.value);
        Ok(cx.string(value_str))
    }

    fn js_get_metadata(mut cx: FunctionContext) -> JsResult<JsValue> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsAstNode>, _>(&mut cx)?;
        let meta = cx.empty_object();
        if let Some(offset) = this.inner.metadata.offset {
            meta.set(&mut cx, "offset", cx.number(offset as f64))?;
        }
        if let Some(size) = this.inner.metadata.size {
            meta.set(&mut cx, "size", cx.number(size as f64))?;
        }
        let warnings = cx.empty_array();
        for (i, warning) in this.inner.metadata.warnings.iter().enumerate() {
            warnings.set(&mut cx, i as u32, cx.string(warning))?;
        }
        meta.set(&mut cx, "warnings", warnings)?;
        meta.set(
            &mut cx,
            "errorCount",
            cx.number(this.inner.metadata.errors.len() as f64),
        )?;
        Ok(meta.upcast())
    }

    fn js_has_property(mut cx: FunctionContext) -> JsResult<JsBoolean> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsAstNode>, _>(&mut cx)?;
        let key = cx.argument::<JsString>(0)?.value(&mut cx);

        let has_prop = match &this.inner.value {
            crate::types::PdfValue::Dictionary(dict) => dict.contains_key(&key),
            _ => false,
        };

        Ok(cx.boolean(has_prop))
    }

    fn js_get_property(mut cx: FunctionContext) -> JsResult<JsValue> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsAstNode>, _>(&mut cx)?;
        let key = cx.argument::<JsString>(0)?.value(&mut cx);

        match &this.inner.value {
            crate::types::PdfValue::Dictionary(dict) => {
                if let Some(value) = dict.get(&key) {
                    let value_str = format!("{:?}", value);
                    Ok(cx.string(value_str).upcast())
                } else {
                    Ok(cx.null().upcast())
                }
            }
            _ => Ok(cx.null().upcast()),
        }
    }
}

/// JavaScript wrapper for ValidationReport
pub struct JsValidationReport {
    inner: Arc<ValidationReport>,
}

impl Finalize for JsValidationReport {}

impl JsValidationReport {
    fn js_is_valid(mut cx: FunctionContext) -> JsResult<JsBoolean> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationReport>, _>(&mut cx)?;
        Ok(cx.boolean(this.inner.is_valid))
    }

    fn js_get_schema_name(mut cx: FunctionContext) -> JsResult<JsString> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationReport>, _>(&mut cx)?;
        Ok(cx.string(&this.inner.schema_name))
    }

    fn js_get_schema_version(mut cx: FunctionContext) -> JsResult<JsString> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationReport>, _>(&mut cx)?;
        Ok(cx.string(&this.inner.schema_version))
    }

    fn js_get_issues(mut cx: FunctionContext) -> JsResult<JsArray> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationReport>, _>(&mut cx)?;
        let js_array = cx.empty_array();

        for (i, issue) in this.inner.issues.iter().enumerate() {
            let js_issue = JsValidationIssue::from_issue(&mut cx, issue.clone())?;
            js_array.set(&mut cx, i as u32, js_issue)?;
        }

        Ok(js_array)
    }

    fn js_get_statistics(mut cx: FunctionContext) -> JsResult<JsObject> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationReport>, _>(&mut cx)?;
        let stats = cx.empty_object();

        stats.set(
            &mut cx,
            "totalChecks",
            cx.number(this.inner.statistics.total_checks),
        )?;
        stats.set(
            &mut cx,
            "passedChecks",
            cx.number(this.inner.statistics.passed_checks),
        )?;
        stats.set(
            &mut cx,
            "failedChecks",
            cx.number(this.inner.statistics.failed_checks),
        )?;
        stats.set(
            &mut cx,
            "infoCount",
            cx.number(this.inner.statistics.info_count),
        )?;
        stats.set(
            &mut cx,
            "warningCount",
            cx.number(this.inner.statistics.warning_count),
        )?;
        stats.set(
            &mut cx,
            "errorCount",
            cx.number(this.inner.statistics.error_count),
        )?;
        stats.set(
            &mut cx,
            "criticalCount",
            cx.number(this.inner.statistics.critical_count),
        )?;

        Ok(stats)
    }
}

/// JavaScript wrapper for ValidationIssue
pub struct JsValidationIssue {
    inner: crate::validation::ValidationIssue,
}

impl Finalize for JsValidationIssue {}

impl JsValidationIssue {
    fn from_issue(
        cx: &mut FunctionContext,
        issue: crate::validation::ValidationIssue,
    ) -> JsResult<JsBox<JsValidationIssue>> {
        let js_issue = JsValidationIssue { inner: issue };
        Ok(cx.boxed(js_issue))
    }

    fn js_get_severity(mut cx: FunctionContext) -> JsResult<JsString> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationIssue>, _>(&mut cx)?;
        let severity = format!("{:?}", this.inner.severity);
        Ok(cx.string(severity))
    }

    fn js_get_code(mut cx: FunctionContext) -> JsResult<JsString> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationIssue>, _>(&mut cx)?;
        Ok(cx.string(&this.inner.code))
    }

    fn js_get_message(mut cx: FunctionContext) -> JsResult<JsString> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationIssue>, _>(&mut cx)?;
        Ok(cx.string(&this.inner.message))
    }

    fn js_get_node_id(mut cx: FunctionContext) -> JsResult<JsValue> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationIssue>, _>(&mut cx)?;

        if let Some(node_id) = this.inner.node_id {
            Ok(cx.number(node_id.0 as f64).upcast())
        } else {
            Ok(cx.null().upcast())
        }
    }

    fn js_get_location(mut cx: FunctionContext) -> JsResult<JsValue> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationIssue>, _>(&mut cx)?;

        if let Some(ref location) = this.inner.location {
            Ok(cx.string(location).upcast())
        } else {
            Ok(cx.null().upcast())
        }
    }

    fn js_get_suggestion(mut cx: FunctionContext) -> JsResult<JsValue> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsValidationIssue>, _>(&mut cx)?;

        if let Some(ref suggestion) = this.inner.suggestion {
            Ok(cx.string(suggestion).upcast())
        } else {
            Ok(cx.null().upcast())
        }
    }
}

/// JavaScript wrapper for PluginManager
pub struct JsPluginManager {
    inner: Arc<Mutex<PluginManager>>,
}

impl Finalize for JsPluginManager {}

impl JsPluginManager {
    fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<JsPluginManager>> {
        let manager = JsPluginManager {
            inner: Arc::new(Mutex::new(PluginManager::new())),
        };
        Ok(cx.boxed(manager))
    }

    fn js_execute_plugins(mut cx: FunctionContext) -> JsResult<JsObject> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPluginManager>, _>(&mut cx)?;
        let js_document = cx.argument::<JsBox<JsPdfDocument>>(0)?;

        let manager = this.inner.lock().unwrap();
        let mut document = js_document.inner.lock().unwrap().clone();
        let summary = manager.execute_plugins(&mut document);

        // Update the document
        *js_document.inner.lock().unwrap() = document;

        let result = cx.empty_object();
        result.set(&mut cx, "totalPlugins", cx.number(summary.total_plugins))?;
        result.set(
            &mut cx,
            "successfulPlugins",
            cx.number(summary.successful_plugins),
        )?;
        result.set(&mut cx, "failedPlugins", cx.number(summary.failed_plugins))?;
        result.set(
            &mut cx,
            "executionTimeMs",
            cx.number(summary.total_execution_time_ms as f64),
        )?;

        // Convert plugin results
        let results_obj = cx.empty_object();
        for (name, result) in summary.plugin_results {
            let result_str = format!("{:?}", result);
            results_obj.set(&mut cx, name.as_str(), cx.string(result_str))?;
        }
        result.set(&mut cx, "pluginResults", results_obj)?;

        Ok(result)
    }

    fn js_list_plugins(mut cx: FunctionContext) -> JsResult<JsArray> {
        let this = cx
            .this()
            .downcast_or_throw::<JsBox<JsPluginManager>, _>(&mut cx)?;
        let manager = this.inner.lock().unwrap();

        let plugins = manager.list_plugins();
        let js_array = cx.empty_array();

        for (i, metadata) in plugins.iter().enumerate() {
            let plugin_obj = cx.empty_object();
            plugin_obj.set(&mut cx, "name", cx.string(&metadata.name))?;
            plugin_obj.set(&mut cx, "version", cx.string(&metadata.version))?;
            plugin_obj.set(&mut cx, "description", cx.string(&metadata.description))?;
            plugin_obj.set(&mut cx, "author", cx.string(&metadata.author))?;

            let tags_array = cx.empty_array();
            for (j, tag) in metadata.tags.iter().enumerate() {
                tags_array.set(&mut cx, j as u32, cx.string(tag))?;
            }
            plugin_obj.set(&mut cx, "tags", tags_array)?;

            js_array.set(&mut cx, i as u32, plugin_obj)?;
        }

        Ok(js_array)
    }
}

/// Module-level functions
fn js_parse_pdf(mut cx: FunctionContext) -> JsResult<JsBox<JsPdfDocument>> {
    let buffer = cx.argument::<JsBuffer>(0)?;
    JsPdfDocument::js_from_buffer(cx)
}

fn js_get_available_schemas(mut cx: FunctionContext) -> JsResult<JsArray> {
    let registry = SchemaRegistry::new();
    let schemas = registry.list_schemas();
    let js_array = cx.empty_array();

    for (i, schema) in schemas.iter().enumerate() {
        js_array.set(&mut cx, i as u32, cx.string(schema))?;
    }

    Ok(js_array)
}

fn js_get_node_types(mut cx: FunctionContext) -> JsResult<JsArray> {
    let types = super::utils::list_node_types();
    let js_array = cx.empty_array();

    for (i, node_type) in types.iter().enumerate() {
        js_array.set(&mut cx, i as u32, cx.string(node_type))?;
    }

    Ok(js_array)
}

/// Initialize the JavaScript module
#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    // PdfDocument class
    cx.export_function("PdfDocument", JsPdfDocument::js_new)?;
    cx.export_function("parseDocument", js_parse_pdf)?;

    // AstNode class methods are attached to instances

    // ValidationReport class methods are attached to instances

    // PluginManager class
    cx.export_function("PluginManager", JsPluginManager::js_new)?;

    // Module-level functions
    cx.export_function("getAvailableSchemas", js_get_available_schemas)?;
    cx.export_function("getNodeTypes", js_get_node_types)?;

    // Constants
    cx.export_value("VERSION", cx.string("0.1.0"))?;
    cx.export_value("AUTHOR", cx.string("PDF-AST Project"))?;

    Ok(())
}
