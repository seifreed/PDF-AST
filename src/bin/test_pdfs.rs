use std::fs;
use std::path::Path;

fn main() {
    println!("Testing PDF files in pdfs/ directory...");

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

    let mut pdf_count = 0;
    let mut total_size = 0u64;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name() {
                if let Ok(metadata) = fs::metadata(&path) {
                    let size = metadata.len();
                    total_size += size;
                    pdf_count += 1;

                    println!("  {} ({} bytes)", filename.to_string_lossy(), size);

                    // Try to read first few bytes to check if it's a PDF
                    match fs::read(&path) {
                        Ok(data) => {
                            if data.len() >= 4 {
                                let header = &data[0..4];
                                if header == b"%PDF" {
                                    println!("    Valid PDF header");
                                } else {
                                    println!("    Warning: No PDF header found: {:?}", header);
                                }
                            } else {
                                println!("    Warning: File too small");
                            }

                            // Look for trailer
                            let data_str = String::from_utf8_lossy(&data);
                            if data_str.contains("trailer") {
                                println!("    Contains trailer");
                            }
                            if data_str.contains("xref") {
                                println!("    Contains xref table");
                            }
                            if data_str.contains("/Root") {
                                println!("    Contains root reference");
                            }
                        }
                        Err(e) => {
                            println!("    Error reading file: {}", e);
                        }
                    }
                }
            }
        }
    }

    println!("\nSummary:");
    println!("  Total files: {}", pdf_count);
    println!(
        "  Total size: {} bytes ({:.2} MB)",
        total_size,
        total_size as f64 / 1024.0 / 1024.0
    );
}
