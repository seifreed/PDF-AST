pub mod object;
pub mod primitive;
pub mod reference;
pub mod stream;

pub use object::*;
pub use primitive::*;
pub use reference::*;
pub use stream::*;

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum PdfValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(PdfString),
    Name(PdfName),
    Array(PdfArray),
    Dictionary(PdfDictionary),
    Stream(PdfStream),
    Reference(PdfReference),
}

impl PdfValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        self.as_bool()
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            PdfValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_real(&self) -> Option<f64> {
        match self {
            PdfValue::Real(r) => Some(*r),
            PdfValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&PdfString> {
        match self {
            PdfValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&PdfName> {
        match self {
            PdfValue::Name(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&PdfArray> {
        match self {
            PdfValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut PdfArray> {
        match self {
            PdfValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&PdfDictionary> {
        match self {
            PdfValue::Dictionary(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_dict_mut(&mut self) -> Option<&mut PdfDictionary> {
        match self {
            PdfValue::Dictionary(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<&PdfStream> {
        match self {
            PdfValue::Stream(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<&PdfReference> {
        match self {
            PdfValue::Reference(r) => Some(r),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            PdfValue::Null => "null",
            PdfValue::Boolean(_) => "boolean",
            PdfValue::Integer(_) => "integer",
            PdfValue::Real(_) => "real",
            PdfValue::String(_) => "string",
            PdfValue::Name(_) => "name",
            PdfValue::Array(_) => "array",
            PdfValue::Dictionary(_) => "dictionary",
            PdfValue::Stream(_) => "stream",
            PdfValue::Reference(_) => "reference",
        }
    }
}

impl fmt::Display for PdfValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfValue::Null => write!(f, "null"),
            PdfValue::Boolean(b) => write!(f, "{}", b),
            PdfValue::Integer(i) => write!(f, "{}", i),
            PdfValue::Real(r) => write!(f, "{}", r),
            PdfValue::String(s) => write!(f, "{}", s),
            PdfValue::Name(n) => write!(f, "{}", n),
            PdfValue::Array(a) => write!(f, "{}", a),
            PdfValue::Dictionary(d) => write!(f, "{}", d),
            PdfValue::Stream(s) => write!(f, "{}", s),
            PdfValue::Reference(r) => write!(f, "{}", r),
        }
    }
}
