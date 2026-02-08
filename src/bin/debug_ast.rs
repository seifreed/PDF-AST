use pdf_ast::parser::reference_resolver::ReferenceResolver;
use pdf_ast::parser::PdfParser;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <pdf_file>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = &args[1];
    println!("Debugging AST generation for: {}", pdf_path);

    // Parse the PDF
    let file = File::open(pdf_path)?;
    let reader = BufReader::new(file);
    let parser = PdfParser::new();
    let mut document = parser.parse(reader)?;

    println!("Initial AST stats:");
    println!("  Nodes: {}", document.ast.node_count());
    println!("  Edges: {}", document.ast.edge_count());

    // Collect references from initial nodes
    let mut total_refs = 0;
    for node in document.ast.get_all_nodes() {
        let refs = extract_references_from_value(&node.value);
        if !refs.is_empty() {
            println!("  Node {}: Found {} references", node.id.0, refs.len());
            for ref_obj in &refs {
                println!(
                    "    -> Object {}/{}",
                    ref_obj.object_number, ref_obj.generation_number
                );
            }
            total_refs += refs.len();
        }
    }
    println!("  Total references found: {}", total_refs);

    // Check what xref entries we have
    println!("Available xref entries:");
    for (obj_id, entry) in &document.xref.entries {
        println!(
            "  Object {}/{}: offset={:?}",
            obj_id.number,
            obj_id.generation,
            match entry {
                pdf_ast::ast::XRefEntry::InUse { offset, .. } => Some(*offset),
                _ => None,
            }
        );
    }

    // Try to create reference resolver
    let file2 = File::open(pdf_path)?;
    let reader2 = BufReader::new(file2);
    println!("\nCreating reference resolver...");

    let mut resolver = ReferenceResolver::from_document(
        reader2,
        &document,
        true,
        pdf_ast::performance::PerformanceLimits::default(),
    );
    println!("Reference resolver created successfully");

    println!("\nResolving references...");
    match resolver.resolve_references(&mut document.ast) {
        Ok(()) => {
            println!("Reference resolution completed");
            println!("Final AST stats:");
            println!("  Nodes: {}", document.ast.node_count());
            println!("  Edges: {}", document.ast.edge_count());

            // Show node types
            println!("\nNode types in final AST:");
            for node in document.ast.get_all_nodes() {
                println!("  Node {}: {:?}", node.id.0, node.node_type);
            }
        }
        Err(e) => {
            eprintln!("Reference resolution failed: {}", e);
        }
    }

    Ok(())
}

fn extract_references_from_value(
    value: &pdf_ast::types::PdfValue,
) -> Vec<pdf_ast::types::PdfReference> {
    use pdf_ast::types::PdfValue;
    let mut references = Vec::new();

    match value {
        PdfValue::Reference(pdf_ref) => {
            references.push(*pdf_ref);
        }
        PdfValue::Array(array) => {
            for item in array.iter() {
                references.extend(extract_references_from_value(item));
            }
        }
        PdfValue::Dictionary(dict) => {
            for (_, val) in dict.iter() {
                references.extend(extract_references_from_value(val));
            }
        }
        PdfValue::Stream(stream) => {
            for (_, val) in stream.dict.iter() {
                references.extend(extract_references_from_value(val));
            }
        }
        _ => {}
    }

    references
}
