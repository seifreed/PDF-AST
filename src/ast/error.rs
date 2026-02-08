use thiserror::Error;

#[derive(Error, Debug)]
pub enum AstError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid reference: object {0} generation {1}")]
    InvalidReference(u32, u16),

    #[error("Invalid reference: {0}")]
    InvalidReferenceString(String),

    #[error("Missing object: {0}")]
    MissingObject(String),

    #[error("Invalid structure: {0}")]
    InvalidStructure(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Filter error: {0}")]
    FilterError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),
}

pub type AstResult<T> = Result<T, AstError>;
