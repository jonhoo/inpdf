use anyhow::{Context, Result};
use lopdf::{Document, Object, ObjectId};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub title: String,
    pub page: Option<u32>,
    pub level: u32,
    pub children: Vec<TocEntry>,
}

/// Extract table of contents / bookmarks from a PDF
pub fn extract_toc<P: AsRef<Path>>(path: P) -> Result<Vec<TocEntry>> {
    let path = path.as_ref();
    let doc =
        Document::load(path).with_context(|| format!("Failed to open PDF: {}", path.display()))?;

    extract_toc_from_doc(&doc)
}

pub fn extract_toc_from_doc(doc: &Document) -> Result<Vec<TocEntry>> {
    // Get the document catalog
    let catalog = doc
        .catalog()
        .with_context(|| "Failed to get document catalog")?;

    // Look for Outlines entry
    let outlines_ref = match catalog.get(b"Outlines") {
        Ok(Object::Reference(r)) => *r,
        _ => return Ok(Vec::new()), // No outlines/bookmarks
    };

    let outlines = match doc.get_dictionary(outlines_ref) {
        Ok(d) => d,
        _ => return Ok(Vec::new()),
    };

    // Get page number mapping
    let page_map = build_page_map(doc);

    // Get first child
    let first_ref = match outlines.get(b"First") {
        Ok(Object::Reference(r)) => *r,
        _ => return Ok(Vec::new()),
    };

    // Parse the outline tree
    let entries = parse_outline_items(doc, first_ref, &page_map, 0)?;

    Ok(entries)
}

fn parse_outline_items(
    doc: &Document,
    first_id: ObjectId,
    page_map: &[(ObjectId, u32)],
    level: u32,
) -> Result<Vec<TocEntry>> {
    let mut entries = Vec::new();
    let mut current_id = Some(first_id);

    while let Some(id) = current_id {
        let dict = match doc.get_dictionary(id) {
            Ok(d) => d,
            Err(_) => break,
        };

        // Get title
        let title = match dict.get(b"Title") {
            Ok(Object::String(bytes, _)) => decode_pdf_string(bytes),
            _ => "Untitled".to_string(),
        };

        // Get destination page
        let page = get_destination_page(doc, dict, page_map);

        // Get children
        let children = match dict.get(b"First") {
            Ok(Object::Reference(child_ref)) => {
                parse_outline_items(doc, *child_ref, page_map, level + 1)?
            }
            _ => Vec::new(),
        };

        entries.push(TocEntry {
            title,
            page,
            level,
            children,
        });

        // Get next sibling
        current_id = match dict.get(b"Next") {
            Ok(Object::Reference(r)) => Some(*r),
            _ => None,
        };
    }

    Ok(entries)
}

fn get_destination_page(
    doc: &Document,
    dict: &lopdf::Dictionary,
    page_map: &[(ObjectId, u32)],
) -> Option<u32> {
    // Try Dest first (direct destination)
    if let Ok(dest) = dict.get(b"Dest") {
        return resolve_destination(doc, dest, page_map);
    }

    // Try A (action) - for GoTo actions
    if let Ok(Object::Reference(action_ref)) = dict.get(b"A") {
        if let Ok(action_dict) = doc.get_dictionary(*action_ref) {
            if let Ok(Object::Name(action_type)) = action_dict.get(b"S") {
                if action_type == b"GoTo" {
                    if let Ok(dest) = action_dict.get(b"D") {
                        return resolve_destination(doc, dest, page_map);
                    }
                }
            }
        }
    }

    // Also check for inline action dictionary
    if let Ok(Object::Dictionary(action_dict)) = dict.get(b"A") {
        if let Ok(Object::Name(action_type)) = action_dict.get(b"S") {
            if action_type == b"GoTo" {
                if let Ok(dest) = action_dict.get(b"D") {
                    return resolve_destination(doc, dest, page_map);
                }
            }
        }
    }

    None
}

fn resolve_destination(doc: &Document, dest: &Object, page_map: &[(ObjectId, u32)]) -> Option<u32> {
    match dest {
        // Named destination - look up in Names/Dests
        Object::String(name, _) | Object::Name(name) => {
            resolve_named_destination(doc, name, page_map)
        }
        // Direct destination array
        Object::Array(arr) => get_page_from_dest_array(arr, page_map),
        // Reference to destination
        Object::Reference(r) => {
            if let Ok(obj) = doc.get_object(*r) {
                resolve_destination(doc, obj, page_map)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn resolve_named_destination(
    doc: &Document,
    name: &[u8],
    page_map: &[(ObjectId, u32)],
) -> Option<u32> {
    // Try to find in Names dictionary
    if let Ok(catalog) = doc.catalog() {
        // Try Names/Dests
        if let Ok(Object::Reference(names_ref)) = catalog.get(b"Names") {
            if let Ok(names_dict) = doc.get_dictionary(*names_ref) {
                if let Ok(Object::Reference(dests_ref)) = names_dict.get(b"Dests") {
                    if let Some(page) = search_name_tree(doc, *dests_ref, name, page_map) {
                        return Some(page);
                    }
                }
            }
        }

        // Try Dests dictionary (older style)
        if let Ok(Object::Reference(dests_ref)) = catalog.get(b"Dests") {
            if let Ok(dests_dict) = doc.get_dictionary(*dests_ref) {
                if let Ok(dest) = dests_dict.get(name) {
                    return resolve_destination(doc, dest, page_map);
                }
            }
        }
    }

    None
}

fn search_name_tree(
    doc: &Document,
    node_id: ObjectId,
    name: &[u8],
    page_map: &[(ObjectId, u32)],
) -> Option<u32> {
    let dict = doc.get_dictionary(node_id).ok()?;

    // Check Names array (leaf node)
    if let Ok(Object::Array(names)) = dict.get(b"Names") {
        for chunk in names.chunks(2) {
            if chunk.len() == 2 {
                if let Object::String(key, _) = &chunk[0] {
                    if key == name {
                        return resolve_destination(doc, &chunk[1], page_map);
                    }
                }
            }
        }
    }

    // Check Kids array (intermediate node)
    if let Ok(Object::Array(kids)) = dict.get(b"Kids") {
        for kid in kids {
            if let Object::Reference(kid_ref) = kid {
                if let Some(page) = search_name_tree(doc, *kid_ref, name, page_map) {
                    return Some(page);
                }
            }
        }
    }

    None
}

fn get_page_from_dest_array(arr: &[Object], page_map: &[(ObjectId, u32)]) -> Option<u32> {
    // Destination array format: [page_ref, /XYZ, left, top, zoom] or similar
    if let Some(Object::Reference(page_ref)) = arr.first() {
        for (id, page_num) in page_map {
            if id == page_ref {
                return Some(*page_num);
            }
        }
    }
    None
}

fn build_page_map(doc: &Document) -> Vec<(ObjectId, u32)> {
    let mut pages: Vec<_> = doc.get_pages().into_iter().collect();
    pages.sort_by_key(|(num, _)| *num);
    pages.into_iter().map(|(num, id)| (id, num)).collect()
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    // Check for UTF-16 BOM
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16 BE
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
        // PDFDocEncoding / Latin-1 (simplified)
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// Flatten TOC entries into a simple list with indentation info
pub fn flatten_toc(entries: &[TocEntry]) -> Vec<FlatTocEntry> {
    let mut result = Vec::new();
    flatten_toc_recursive(entries, &mut result);
    result
}

fn flatten_toc_recursive(entries: &[TocEntry], result: &mut Vec<FlatTocEntry>) {
    for entry in entries {
        result.push(FlatTocEntry {
            title: entry.title.clone(),
            page: entry.page,
            level: entry.level,
        });
        flatten_toc_recursive(&entry.children, result);
    }
}

#[derive(Debug, Clone)]
pub struct FlatTocEntry {
    pub title: String,
    pub page: Option<u32>,
    pub level: u32,
}
