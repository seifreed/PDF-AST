use crate::ast::{AstResult, PdfDocument};
use crate::parser::pdf_file::PdfFileParser;
use crate::performance::PerformanceLimits;
use std::io::{BufRead, Read, Seek};

pub struct DocumentParser<R: Read + Seek + BufRead> {
    reader: R,
    tolerant: bool,
    limits: PerformanceLimits,
}

impl<R: Read + Seek + BufRead> DocumentParser<R> {
    pub fn new(reader: R, tolerant: bool, limits: PerformanceLimits) -> Self {
        DocumentParser {
            reader,
            tolerant,
            limits,
        }
    }

    pub fn parse(self) -> AstResult<PdfDocument> {
        let parser = PdfFileParser::new(self.reader, self.tolerant, self.limits)?;
        parser.parse()
    }
}
