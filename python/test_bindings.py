#!/usr/bin/env python3
"""Test script for PDF-AST Python bindings"""

import pdf_ast

def test_basic_functionality():
    """Test basic functionality of PDF-AST Python bindings"""
    # Test with a minimal PDF
    minimal_pdf = b"""%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
>>
endobj
xref
0 4
0000000000 65535 f 
0000000010 00000 n 
0000000079 00000 n 
0000000136 00000 n 
trailer
<<
/Size 4
/Root 1 0 R
>>
startxref
200
%%EOF"""
    
    print("\n=== Testing PDF parsing ===")
    try:
        doc = pdf_ast.parse_pdf(minimal_pdf)
        print(f"Document: {doc}")
        print(f"Version: {doc.get_version()}")
        stats = doc.get_statistics()
        print(f"Statistics: {dict(stats)}")
        
    except Exception as e:
        print(f"Error parsing PDF: {e}")
    
    print("\n=== Testing validation ===")
    try:
        schemas = pdf_ast.get_available_schemas()
        print(f"Schemas: {schemas}")
        if "PDF-2.0" in schemas:
            report = doc.validate("PDF-2.0")
            print(f"Validation report: {report}")
            print(f"Is valid: {report.is_valid()}")
            print(f"Issues: {len(report.get_issues())}")
    except Exception as e:
        print(f"Error validating PDF: {e}")
    
    print("\n=== Testing invalid PDF ===")
    try:
        invalid_data = b"This is not a PDF file"
        result = pdf_ast.is_pdf(invalid_data)
        print(f"Is invalid data PDF: {result}")
        
        # This should raise an error
        doc = pdf_ast.parse_pdf(invalid_data)
        print("ERROR: Should have failed!")
        
    except Exception as e:
        print(f"Expected error for invalid PDF: {e}")

if __name__ == "__main__":
    test_basic_functionality()
    print("\n✅ All tests completed!")
