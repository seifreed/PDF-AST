use crate::types::{ObjectId, PdfArray, PdfDictionary, PdfStream, PdfValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AstNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub value: PdfValue,
    pub metadata: NodeMetadata,
    pub children: Vec<NodeId>,
    pub references: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub usize);

impl NodeId {
    pub fn new(id: usize) -> Self {
        NodeId(id)
    }

    /// Returns the underlying index value for this node ID.
    pub fn index(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Root,
    Catalog,
    Pages,
    Page,
    Resource,
    Font,
    Image,
    ContentStream,
    Annotation,
    Action,
    Metadata,
    EmbeddedFile,
    Signature,
    Object(ObjectId),
    Unknown,
    // Stream types
    Stream,
    FilteredStream,
    DecodedStream,
    // XObject types
    XObject,
    FormXObject,
    ImageXObject,
    // Font types
    Type1Font,
    TrueTypeFont,
    Type3Font,
    CIDFont,
    // Action types
    JavaScriptAction,
    GoToAction,
    URIAction,
    LaunchAction,
    SubmitFormAction,
    // Form types
    AcroForm,
    Field,
    // Security types
    Encrypt,
    Permission,
    // Content stream operators
    ContentOperator,
    TextOperator,
    GraphicsOperator,
    // Suspicious elements
    EmbeddedJS,
    SuspiciousAction,
    ExternalReference,
    EncodedContent,
    // Additional structure types
    Outline,
    OutlineItem,
    NameTree,
    StructTreeRoot,
    StructElem,
    // Graphics and color
    ColorSpace,
    ICCBased,
    Separation,
    DeviceN,
    Indexed,
    Pattern,
    Shading,
    ExtGState,
    Function,
    // CMap and encoding
    CMap,
    ToUnicode,
    Encoding,
    // Optional content
    OCG,
    OCProperties,
    OCMD,
    // Multimedia
    RichMedia,
    Rendition,
    Screen,
    Sound,
    Movie,
    // 3D
    ThreeD,
    U3D,
    PRC,
    // Output intents
    OutputIntent,
    // Annotations subtypes
    LinkAnnotation,
    WidgetAnnotation,
    FileAttachmentAnnotation,
    // Inline images
    InlineImage,
    // Missing variants
    Form,
    Structure,
    Multimedia,
    JavaScript,
    Encryption,
    Content,
    Other,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub offset: Option<u64>,
    pub size: Option<usize>,
    pub errors: Vec<ParseError>,
    pub warnings: Vec<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    pub code: ErrorCode,
    pub message: String,
    pub offset: Option<u64>,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    InvalidSyntax,
    MissingObject,
    InvalidReference,
    CorruptedStream,
    InvalidFilter,
    UnsupportedFeature,
    MalformedStructure,
}

impl AstNode {
    pub fn new(id: NodeId, node_type: NodeType, value: PdfValue) -> Self {
        AstNode {
            id,
            node_type,
            value,
            metadata: NodeMetadata::default(),
            children: Vec::new(),
            references: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: NodeMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn add_child(&mut self, child_id: NodeId) {
        self.children.push(child_id);
    }

    pub fn add_reference(&mut self, ref_id: NodeId) {
        self.references.push(ref_id);
    }

    pub fn is_error(&self) -> bool {
        !self.metadata.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.metadata.warnings.is_empty()
    }

    pub fn as_dict(&self) -> Option<&PdfDictionary> {
        self.value.as_dict()
    }

    pub fn as_array(&self) -> Option<&PdfArray> {
        self.value.as_array()
    }

    pub fn as_stream(&self) -> Option<&PdfStream> {
        self.value.as_stream()
    }
}

impl NodeType {
    pub fn from_dict(dict: &PdfDictionary) -> Self {
        if let Some(type_name) = dict.get_type() {
            match type_name.without_slash() {
                "Catalog" => NodeType::Catalog,
                "Pages" => NodeType::Pages,
                "Page" => NodeType::Page,
                "Font" => NodeType::Font,
                "XObject" => {
                    if let Some(subtype) = dict.get_subtype() {
                        match subtype.without_slash() {
                            "Image" => NodeType::Image,
                            _ => NodeType::Resource,
                        }
                    } else {
                        NodeType::Resource
                    }
                }
                "Annot" => NodeType::Annotation,
                "Action" => NodeType::Action,
                "Metadata" => NodeType::Metadata,
                "Filespec" => NodeType::EmbeddedFile,
                "Sig" => NodeType::Signature,
                _ => NodeType::Unknown,
            }
        } else {
            NodeType::Unknown
        }
    }
}

impl NodeMetadata {
    pub fn new() -> Self {
        NodeMetadata::default()
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn set_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.properties.get(key)
    }
}

impl ParseError {
    pub fn new(code: ErrorCode, message: String) -> Self {
        ParseError {
            code,
            message,
            offset: None,
            recoverable: false,
        }
    }

    pub fn recoverable(mut self) -> Self {
        self.recoverable = true;
        self
    }

    pub fn at_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}
