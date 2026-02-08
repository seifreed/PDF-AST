use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PdfReference {
    pub object_number: u32,
    pub generation_number: u16,
}

impl PdfReference {
    pub fn new(object_number: u32, generation_number: u16) -> Self {
        PdfReference {
            object_number,
            generation_number,
        }
    }

    pub fn id(&self) -> ObjectId {
        ObjectId {
            number: self.object_number,
            generation: self.generation_number,
        }
    }

    pub fn number(&self) -> u32 {
        self.object_number
    }

    pub fn generation(&self) -> u16 {
        self.generation_number
    }

    pub fn object_id(&self) -> ObjectId {
        self.id()
    }
}

impl fmt::Display for PdfReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.object_number, self.generation_number)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId {
    pub number: u32,
    pub generation: u16,
}

impl ObjectId {
    pub fn new(number: u32, generation: u16) -> Self {
        ObjectId { number, generation }
    }

    pub fn to_reference(&self) -> PdfReference {
        PdfReference::new(self.number, self.generation)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} obj", self.number, self.generation)
    }
}

impl From<(u32, u16)> for ObjectId {
    fn from((number, generation): (u32, u16)) -> Self {
        ObjectId::new(number, generation)
    }
}

impl From<ObjectId> for PdfReference {
    fn from(id: ObjectId) -> Self {
        id.to_reference()
    }
}

impl From<PdfReference> for ObjectId {
    fn from(reference: PdfReference) -> Self {
        reference.id()
    }
}

impl From<&PdfReference> for ObjectId {
    fn from(reference: &PdfReference) -> Self {
        reference.id()
    }
}

impl From<&ObjectId> for PdfReference {
    fn from(id: &ObjectId) -> Self {
        id.to_reference()
    }
}
