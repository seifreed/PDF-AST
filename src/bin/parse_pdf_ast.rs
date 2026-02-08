use pdf_ast::parser::PdfParser;
use pdf_ast::serialization::to_json;
use std::fs;
use std::io::BufReader;
use std::path::Path;

fn main() {
    println!("PDF AST Parser - Converting PDFs to AST JSON format");
    println!("=================================================\n");

    let pdfs_dir = "pdfs";

    if !Path::new(pdfs_dir).exists() {
        println!("Error: pdfs/ directory not found");
        return;
    }

    let entries = match fs::read_dir(pdfs_dir) {
        Ok(entries) => entries,
        Err(e) => {
            println!("Error reading pdfs directory: {}", e);
            return;
        }
    };

    let mut processed = 0;
    let mut successful = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            processed += 1;

            if let Some(filename) = path.file_name() {
                println!("Processing: {}", filename.to_string_lossy());

                match parse_pdf_to_ast(&path) {
                    Ok(json) => {
                        successful += 1;
                        println!("  ✓ Successfully parsed to AST");

                        // Show first 500 characters of JSON
                        let preview = if json.len() > 500 {
                            format!("{}...", &json[..500])
                        } else {
                            json.clone()
                        };

                        println!("  AST Preview:");
                        println!("  {}\n", preview);

                        // Save full AST to file
                        let output_path = format!("output/{}.ast.json", filename.to_string_lossy());
                        if let Err(e) = fs::create_dir_all("output") {
                            println!("  Warning: Could not create output directory: {}", e);
                        } else if let Err(e) = fs::write(&output_path, &json) {
                            println!("  Warning: Could not save AST to {}: {}", output_path, e);
                        } else {
                            println!("  💾 Full AST saved to: {}", output_path);
                        }
                    }
                    Err(e) => {
                        println!("  ✗ Error parsing: {}", e);
                    }
                }
                println!("  {}", "-".repeat(60));
            }
        }
    }

    println!("\nSummary:");
    println!("  Total files processed: {}", processed);
    println!("  Successfully parsed: {}", successful);
    println!(
        "  Success rate: {:.1}%",
        (successful as f64 / processed as f64) * 100.0
    );

    if successful > 0 {
        println!("\n✓ AST JSON files saved in output/ directory");
    }
}

fn parse_pdf_to_ast(path: &Path) -> Result<String, String> {
    // Read the PDF file
    let file = fs::File::open(path).map_err(|e| format!("Could not open file: {}", e))?;

    let buf_reader = BufReader::new(file);

    // Create parser
    let parser = PdfParser::new();

    // Parse the PDF into AST
    let document = parser
        .parse(buf_reader)
        .map_err(|e| format!("Parse error: {:?}", e))?;

    // Convert AST to JSON
    let json = to_json(&document).map_err(|e| format!("JSON serialization error: {:?}", e))?;

    Ok(json)
}
