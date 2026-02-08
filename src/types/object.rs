use crate::types::{PdfName, PdfValue};
use indexmap::IndexMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfArray {
    elements: Vec<PdfValue>,
}

impl PdfArray {
    pub fn new() -> Self {
        PdfArray {
            elements: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        PdfArray {
            elements: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: PdfValue) {
        self.elements.push(value);
    }

    pub fn get(&self, index: usize) -> Option<&PdfValue> {
        self.elements.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut PdfValue> {
        self.elements.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PdfValue> {
        self.elements.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PdfValue> {
        self.elements.iter_mut()
    }

    pub fn into_vec(self) -> Vec<PdfValue> {
        self.elements
    }

    pub fn as_slice(&self) -> &[PdfValue] {
        &self.elements
    }

    pub fn truncate(&mut self, len: usize) {
        self.elements.truncate(len);
    }
}

impl std::ops::Index<usize> for PdfArray {
    type Output = PdfValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

impl<'a> IntoIterator for &'a PdfArray {
    type Item = &'a PdfValue;
    type IntoIter = std::slice::Iter<'a, PdfValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

impl fmt::Display for PdfArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, elem) in self.elements.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", elem)?;
        }
        write!(f, "]")
    }
}

impl From<Vec<PdfValue>> for PdfArray {
    fn from(elements: Vec<PdfValue>) -> Self {
        PdfArray { elements }
    }
}

impl IntoIterator for PdfArray {
    type Item = PdfValue;
    type IntoIter = std::vec::IntoIter<PdfValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfDictionary {
    entries: IndexMap<PdfName, PdfValue>,
}

impl PdfDictionary {
    pub fn new() -> Self {
        PdfDictionary {
            entries: IndexMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        PdfDictionary {
            entries: IndexMap::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, key: impl Into<PdfName>, value: PdfValue) -> Option<PdfValue> {
        self.entries.insert(key.into(), value)
    }

    pub fn get(&self, key: &str) -> Option<&PdfValue> {
        self.entries.get(&PdfName::new(key))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut PdfValue> {
        self.entries.get_mut(&PdfName::new(key))
    }

    pub fn get_name(&self, key: &PdfName) -> Option<&PdfValue> {
        self.entries.get(key)
    }

    pub fn get_name_mut(&mut self, key: &PdfName) -> Option<&mut PdfValue> {
        self.entries.get_mut(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<PdfValue> {
        self.entries.swap_remove(&PdfName::new(key))
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(&PdfName::new(key))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PdfName, &PdfValue)> {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&PdfName, &mut PdfValue)> {
        self.entries.iter_mut()
    }

    pub fn keys(&self) -> impl Iterator<Item = &PdfName> {
        self.entries.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &PdfValue> {
        self.entries.values()
    }

    pub fn get_type(&self) -> Option<&PdfName> {
        self.get("Type").and_then(|v| v.as_name())
    }

    pub fn get_subtype(&self) -> Option<&PdfName> {
        self.get("Subtype").and_then(|v| v.as_name())
    }

    pub fn into_map(self) -> IndexMap<PdfName, PdfValue> {
        self.entries
    }

    pub fn entry(
        &mut self,
        key: impl Into<PdfName>,
    ) -> indexmap::map::Entry<'_, PdfName, PdfValue> {
        self.entries.entry(key.into())
    }
}

impl<'a> IntoIterator for &'a PdfDictionary {
    type Item = (&'a PdfName, &'a PdfValue);
    type IntoIter = indexmap::map::Iter<'a, PdfName, PdfValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl fmt::Display for PdfDictionary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<<")?;
        for (i, (key, value)) in self.entries.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{} {}", key, value)?;
        }
        write!(f, ">>")
    }
}

impl From<IndexMap<PdfName, PdfValue>> for PdfDictionary {
    fn from(entries: IndexMap<PdfName, PdfValue>) -> Self {
        PdfDictionary { entries }
    }
}

impl IntoIterator for PdfDictionary {
    type Item = (PdfName, PdfValue);
    type IntoIter = indexmap::map::IntoIter<PdfName, PdfValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}
