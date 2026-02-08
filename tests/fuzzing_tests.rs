use pdf_ast::parser::PdfParser;
use std::fs;
use std::path::PathBuf;

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_usize(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next() as usize) % max
    }
}

fn mutate_bytes(mut data: Vec<u8>, rng: &mut XorShift64, flips: usize) -> Vec<u8> {
    let len = data.len();
    if len == 0 {
        return data;
    }
    for _ in 0..flips {
        let pos = rng.next_usize(len);
        let bit = 1u8 << (rng.next_usize(8) as u8);
        data[pos] ^= bit;
    }
    data
}

fn corpus_files() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let dir = PathBuf::from("pdfs");
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
                paths.push(path);
            }
        }
    }
    paths
}

#[test]
fn fuzz_mutated_pdfs_do_not_panic() {
    let parser = PdfParser::new();
    let mut rng = XorShift64::new(0xC0FFEE);
    let files = corpus_files();

    if files.is_empty() {
        return;
    }

    for path in files.into_iter().take(10) {
        let data = fs::read(&path).expect("read pdf");
        let flips = std::cmp::max(1, data.len() / 1000);

        for _ in 0..5 {
            let mutated = mutate_bytes(data.clone(), &mut rng, flips);
            let _ = parser.parse_bytes(&mutated);
        }

        // Truncated variant
        if data.len() > 20 {
            let truncated = data[..data.len() / 2].to_vec();
            let _ = parser.parse_bytes(&truncated);
        }
    }
}
