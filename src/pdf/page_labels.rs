use anyhow::{Context, Result};
use lopdf::{Document, Object};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PageLabel {
    pub physical_page: u32,
    pub logical_label: String,
}

#[derive(Debug, Clone)]
struct PageLabelRange {
    start_page: u32,   // 0-indexed physical page where this range starts
    style: LabelStyle, // Numbering style
    prefix: String,    // Optional prefix
    start_value: u32,  // Starting value for this range
}

#[derive(Debug, Clone, Copy)]
enum LabelStyle {
    Decimal,    // D: 1, 2, 3, ...
    LowerRoman, // r: i, ii, iii, iv, ...
    UpperRoman, // R: I, II, III, IV, ...
    LowerAlpha, // a: a, b, c, ... z, aa, ab, ...
    UpperAlpha, // A: A, B, C, ... Z, AA, AB, ...
    None,       // No numbering, just prefix
}

/// Extract page label mapping from a PDF
pub fn extract_page_labels<P: AsRef<Path>>(path: P) -> Result<Vec<PageLabel>> {
    let path = path.as_ref();
    let doc =
        Document::load(path).with_context(|| format!("Failed to open PDF: {}", path.display()))?;

    extract_page_labels_from_doc(&doc)
}

pub fn extract_page_labels_from_doc(doc: &Document) -> Result<Vec<PageLabel>> {
    let total_pages = doc.get_pages().len() as u32;

    // Get the document catalog
    let catalog = doc.catalog()?;

    // Look for PageLabels entry
    let page_labels_ref = match catalog.get(b"PageLabels") {
        Ok(Object::Reference(r)) => *r,
        Ok(Object::Dictionary(_)) => {
            // Inline dictionary - not common but handle it
            return generate_default_labels(total_pages);
        }
        _ => return generate_default_labels(total_pages),
    };

    let page_labels_dict = match doc.get_dictionary(page_labels_ref) {
        Ok(d) => d,
        _ => return generate_default_labels(total_pages),
    };

    // PageLabels uses a number tree structure
    let ranges = parse_number_tree(doc, page_labels_dict)?;

    // Generate labels for all pages
    let mut labels = Vec::new();
    for physical_page in 1..=total_pages {
        let label = compute_label(&ranges, physical_page - 1); // ranges use 0-indexed pages
        labels.push(PageLabel {
            physical_page,
            logical_label: label,
        });
    }

    Ok(labels)
}

fn parse_number_tree(doc: &Document, dict: &lopdf::Dictionary) -> Result<Vec<PageLabelRange>> {
    let mut ranges = Vec::new();

    // Check for Nums array (leaf node)
    if let Ok(Object::Array(nums)) = dict.get(b"Nums") {
        parse_nums_array(doc, nums, &mut ranges)?;
    }

    // Check for Kids array (intermediate node)
    if let Ok(Object::Array(kids)) = dict.get(b"Kids") {
        for kid in kids {
            if let Object::Reference(kid_ref) = kid {
                if let Ok(kid_dict) = doc.get_dictionary(*kid_ref) {
                    let child_ranges = parse_number_tree(doc, kid_dict)?;
                    ranges.extend(child_ranges);
                }
            }
        }
    }

    // Sort by start page
    ranges.sort_by_key(|r| r.start_page);

    Ok(ranges)
}

fn parse_nums_array(
    doc: &Document,
    nums: &[Object],
    ranges: &mut Vec<PageLabelRange>,
) -> Result<()> {
    // Nums array format: [page_index, label_dict, page_index, label_dict, ...]
    for chunk in nums.chunks(2) {
        if chunk.len() != 2 {
            continue;
        }

        let start_page = match &chunk[0] {
            Object::Integer(n) => *n as u32,
            _ => continue,
        };

        let label_dict = match &chunk[1] {
            Object::Dictionary(d) => d,
            Object::Reference(r) => match doc.get_dictionary(*r) {
                Ok(d) => d,
                _ => continue,
            },
            _ => continue,
        };

        let style = match label_dict.get(b"S") {
            Ok(Object::Name(name)) => match name.as_slice() {
                b"D" => LabelStyle::Decimal,
                b"r" => LabelStyle::LowerRoman,
                b"R" => LabelStyle::UpperRoman,
                b"a" => LabelStyle::LowerAlpha,
                b"A" => LabelStyle::UpperAlpha,
                _ => LabelStyle::Decimal,
            },
            _ => LabelStyle::None,
        };

        let prefix = match label_dict.get(b"P") {
            Ok(Object::String(bytes, _)) => decode_pdf_string(bytes),
            _ => String::new(),
        };

        let start_value = match label_dict.get(b"St") {
            Ok(Object::Integer(n)) => *n as u32,
            _ => 1,
        };

        ranges.push(PageLabelRange {
            start_page,
            style,
            prefix,
            start_value,
        });
    }

    Ok(())
}

fn compute_label(ranges: &[PageLabelRange], page_index: u32) -> String {
    // Find the applicable range for this page
    let range = ranges
        .iter()
        .rev()
        .find(|r| r.start_page <= page_index)
        .cloned()
        .unwrap_or(PageLabelRange {
            start_page: 0,
            style: LabelStyle::Decimal,
            prefix: String::new(),
            start_value: 1,
        });

    let offset = page_index - range.start_page;
    let value = range.start_value + offset;

    let number_part = match range.style {
        LabelStyle::Decimal => value.to_string(),
        LabelStyle::LowerRoman => to_roman(value).to_lowercase(),
        LabelStyle::UpperRoman => to_roman(value),
        LabelStyle::LowerAlpha => to_alpha(value).to_lowercase(),
        LabelStyle::UpperAlpha => to_alpha(value),
        LabelStyle::None => String::new(),
    };

    format!("{}{}", range.prefix, number_part)
}

fn to_roman(mut n: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }

    let values = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];

    let mut result = String::new();
    for (value, numeral) in values {
        while n >= value {
            result.push_str(numeral);
            n -= value;
        }
    }
    result
}

fn to_alpha(n: u32) -> String {
    if n == 0 {
        return String::new();
    }

    let mut result = String::new();
    let mut remaining = n - 1; // Convert to 0-indexed

    loop {
        let letter = ((remaining % 26) as u8 + b'A') as char;
        result.insert(0, letter);
        if remaining < 26 {
            break;
        }
        remaining = remaining / 26 - 1;
    }

    result
}

fn generate_default_labels(total_pages: u32) -> Result<Vec<PageLabel>> {
    Ok((1..=total_pages)
        .map(|p| PageLabel {
            physical_page: p,
            logical_label: p.to_string(),
        })
        .collect())
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    // Check for UTF-16 BOM
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let u16_chars: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    Some(u16::from_be_bytes([chunk[0], chunk[1]]))
                } else {
                    None
                }
            })
            .collect();
        String::from_utf16_lossy(&u16_chars)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_roman() {
        assert_eq!(to_roman(1), "I");
        assert_eq!(to_roman(4), "IV");
        assert_eq!(to_roman(9), "IX");
        assert_eq!(to_roman(42), "XLII");
        assert_eq!(to_roman(1999), "MCMXCIX");
    }

    #[test]
    fn test_to_alpha() {
        assert_eq!(to_alpha(1), "A");
        assert_eq!(to_alpha(26), "Z");
        assert_eq!(to_alpha(27), "AA");
        assert_eq!(to_alpha(28), "AB");
    }
}
