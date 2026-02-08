<p align="center">
  <img src="https://img.shields.io/badge/PDF--AST-PDF%20Security%20Analysis-blue?style=for-the-badge" alt="PDF-AST">
</p>

<h1 align="center">PDF-AST</h1>

<p align="center">
  <strong>Universal Abstract Syntax Tree for PDF documents, built for security analysis and deep inspection</strong>
</p>

<p align="center">
  <a href="https://github.com/seifreed/PDF-AST/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License"></a>
  <a href="https://github.com/seifreed/PDF-AST/actions"><img src="https://img.shields.io/github/actions/workflow/status/seifreed/PDF-AST/ci.yml?style=flat-square&logo=github&label=CI" alt="CI Status"></a>
  <img src="https://img.shields.io/badge/rust-stable-orange?style=flat-square" alt="Rust Stable">
</p>

<p align="center">
  <a href="https://github.com/seifreed/PDF-AST/stargazers"><img src="https://img.shields.io/github/stars/seifreed/PDF-AST?style=flat-square" alt="GitHub Stars"></a>
  <a href="https://github.com/seifreed/PDF-AST/issues"><img src="https://img.shields.io/github/issues/seifreed/PDF-AST?style=flat-square" alt="GitHub Issues"></a>
</p>

---

## Overview

**PDF-AST** is a comprehensive Rust library and toolchain for parsing PDF documents into a rich Abstract Syntax Tree (AST). It is designed for security analysis, malware detection, compliance validation, and structured document inspection. The parser targets **ISO 32000-2 (PDF 2.0)** while maintaining backward compatibility with older PDFs.

### Key Features

| Feature | Description |
|---------|-------------|
| **Full PDF 2.0 Coverage** | Objects, streams, xref tables, linearization, and incremental updates |
| **Rich AST** | 70+ node types representing the full PDF object graph |
| **Security Analysis** | JavaScript, forms, embedded files, and suspicious actions |
| **PDF/A Validation** | Compliance checks for archival standards |
| **Stream Decoding** | Flate, LZW, CCITT, DCT, JPX, and more |
| **XFA + AcroForm** | XML packets, scripts, and field trees |
| **Signature Support** | CMS/PKCS#7 parsing and optional crypto verification |
| **CLI + Library** | Use from the command line or embed in Rust apps |

---

## Supported Use Cases

- **Threat Intelligence**: detect suspicious actions, embedded scripts, and attachments.
- **Malware Research**: inspect object graphs and content streams for obfuscation.
- **Compliance & Archival**: validate PDF/A structure and conformance.
- **Forensics**: extract full AST for offline analysis and correlation.
- **Pipeline Integration**: run automated PDF parsing in CI/CD or batch jobs.

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
pdf-ast = { git = "https://github.com/seifreed/PDF-AST" }
```

### Feature Flags

```toml
[dependencies]
pdf-ast = {
    git = "https://github.com/seifreed/PDF-AST",
    features = ["crypto", "parallel", "async"]
}
```

Available features:
- `crypto`: cryptographic support (signatures, encryption, timestamps, OCSP/CRL)
- `parallel`: multi-threading with Rayon
- `async`: async parsing with Tokio
- `python`: Python bindings via PyO3
- `javascript`: Node.js bindings via Neon
- `full`: all features enabled

### OpenSSL (for `crypto`)

The `crypto` feature requires OpenSSL headers and libraries. On Windows, set `OPENSSL_DIR`, `OPENSSL_LIB_DIR`, and `OPENSSL_INCLUDE_DIR` if OpenSSL is not in a standard location.

---

## Quick Start

```bash
# Build CLI tools
cargo build --release

# Parse a PDF into JSON AST
./target/release/pdf-ast-simple parse document.pdf -o output.json

# Analyze security signals
./target/release/pdf-ast-simple analyze document.pdf --detailed
```

---

## CLI Usage

### pdf-ast-simple (Production Ready)

```bash
# Parse to AST JSON
pdf-ast-simple parse document.pdf -o output.json

# Security analysis
pdf-ast-simple analyze document.pdf --detailed

# Benchmark parsing
pdf-ast-simple benchmark large-file.pdf -i 10
```

### pdf-ast (Advanced)

```bash
# Parse with full options
pdf-ast parse input.pdf --include-streams --resolve-refs

# PDF/A validation
pdf-ast validate input.pdf --schema pdf-a-1b --strict

# Security analysis
pdf-ast analyze input.pdf --security --metrics

# TSA controls for RFC3161 timestamps
pdf-ast analyze input.pdf --security --tsa-allow-fingerprint <SHA256>
pdf-ast analyze input.pdf --security --disable-tsa-revocation-checks

# Security report output formats
pdf-ast analyze input.pdf --security --format yaml
pdf-ast analyze input.pdf --security --format toml

# Write security report to a file
pdf-ast analyze input.pdf --security --security-report report.json
```

---

## Library Usage

### Basic Parsing

```rust
use pdf_ast::{PdfParser, PdfDocument};
use std::fs::File;

let mut file = File::open("document.pdf")?;
let parser = PdfParser::new();
let document: PdfDocument = parser.parse(&mut file)?;

println!("PDF Version: {}", document.version);
println!("Object Count: {}", document.ast.node_count());
```

### Security Analysis

```rust
use pdf_ast::{PdfParser, security::SecurityAnalyzer};
use std::fs::File;

let mut file = File::open("document.pdf")?;
let parser = PdfParser::new();
let document = parser.parse(&mut file)?;

let analyzer = SecurityAnalyzer::new();
let report = analyzer.analyze(&document);
println!("Security Score: {}/100", report.score);
```

### Working with the AST

```rust
use pdf_ast::ast::{NodeType, EdgeType};

let catalog = document.ast.find_nodes(|node| {
    matches!(node.node_type, NodeType::Catalog)
}).next().unwrap();

for edge in document.ast.edges_from(catalog.id) {
    if edge.edge_type == EdgeType::Reference {
        let target = document.ast.get_node(edge.target).unwrap();
        println!("Catalog references: {:?}", target.node_type);
    }
}
```

---

## Project Structure

```
PDF-AST/
├── src/            # Core library
├── tests/          # Test suite
├── examples/       # Usage examples
├── include/        # C header (pdf_ast.h)
└── scripts/        # Utilities and helpers
```

---

## Contributing

Contributions are welcome:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---


## Support the Project

If you find PDF-AST useful, consider supporting its development:

<a href="https://buymeacoffee.com/seifreed" target="_blank">
  <img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50">
</a>

---

<p align="center">
  <sub>Built for secure PDF analysis and research</sub>
</p>
