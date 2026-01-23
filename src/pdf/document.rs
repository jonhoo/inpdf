use anyhow::{Context, Result};
use lopdf::{Document, Object, ObjectId};
use std::path::Path;

pub struct PdfDocument {
    pub doc: Document,
    #[allow(dead_code)]
    pub path: String,
}

impl PdfDocument {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();
        let doc =
            Document::load(&path).with_context(|| format!("Failed to open PDF: {}", path_str))?;
        Ok(PdfDocument {
            doc,
            path: path_str,
        })
    }

    pub fn page_count(&self) -> u32 {
        self.doc.get_pages().len() as u32
    }

    /// Get 1-indexed page object IDs
    pub fn page_ids(&self) -> Vec<(u32, ObjectId)> {
        let mut pages: Vec<_> = self.doc.get_pages().into_iter().collect();
        pages.sort_by_key(|(num, _)| *num);
        pages
    }

    /// Get metadata from the document info dictionary
    pub fn get_info(&self) -> PdfInfo {
        let mut info = PdfInfo::default();

        if let Ok(info_obj) = self.doc.trailer.get(b"Info") {
            if let Object::Reference(info_ref) = info_obj {
                if let Ok(Object::Dictionary(dict)) = self.doc.get_object(*info_ref) {
                    info.title = get_string_from_dict(dict, b"Title");
                    info.author = get_string_from_dict(dict, b"Author");
                    info.creator = get_string_from_dict(dict, b"Creator");
                    info.producer = get_string_from_dict(dict, b"Producer");
                    info.creation_date = get_string_from_dict(dict, b"CreationDate");
                    info.mod_date = get_string_from_dict(dict, b"ModDate");
                    info.subject = get_string_from_dict(dict, b"Subject");
                    info.keywords = get_string_from_dict(dict, b"Keywords");
                }
            }
        }

        info.page_count = self.page_count();
        info
    }

    /// Extract specific pages to a new document
    pub fn extract_pages(&self, pages: &[u32]) -> Result<Document> {
        let mut new_doc = self.doc.clone();
        let all_pages = self.page_ids();
        let total = all_pages.len() as u32;

        // Validate page numbers
        for &page in pages {
            if page == 0 || page > total {
                anyhow::bail!("Page {} is out of range (1-{})", page, total);
            }
        }

        // Get page numbers to delete (pages NOT in our list)
        let pages_to_delete: Vec<u32> = all_pages
            .iter()
            .filter(|(num, _)| !pages.contains(num))
            .map(|(num, _)| *num)
            .collect();

        // Delete pages not in our list
        if !pages_to_delete.is_empty() {
            new_doc.delete_pages(&pages_to_delete);
        }

        Ok(new_doc)
    }

    /// Save to a file
    pub fn save<P: AsRef<Path>>(doc: &mut Document, path: P) -> Result<()> {
        doc.save(&path)
            .with_context(|| format!("Failed to save PDF: {}", path.as_ref().display()))?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct PdfInfo {
    pub title: Option<String>,
    pub author: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub page_count: u32,
}

fn get_string_from_dict(dict: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
    dict.get(key).ok().and_then(|obj| match obj {
        Object::String(bytes, _) => decode_pdf_string(bytes),
        _ => None,
    })
}

fn decode_pdf_string(bytes: &[u8]) -> Option<String> {
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
        String::from_utf16(&u16_chars).ok()
    } else {
        // Try as Latin-1 / PDFDocEncoding (simplified)
        Some(bytes.iter().map(|&b| b as char).collect())
    }
}
