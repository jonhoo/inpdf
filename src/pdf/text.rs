use anyhow::{Context, Result};
use std::path::Path;

/// Extract text from all pages of a PDF
#[allow(dead_code)]
pub fn extract_text<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read PDF: {}", path.display()))?;

    pdf_extract::extract_text_from_mem(&bytes)
        .with_context(|| format!("Failed to extract text from PDF: {}", path.display()))
}

/// Extract text from specific pages of a PDF
pub fn extract_text_pages<P: AsRef<Path>>(path: P, pages: &[u32]) -> Result<Vec<PageText>> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read PDF: {}", path.display()))?;

    // pdf-extract doesn't have per-page extraction in its simple API
    // We'll use lopdf to get page count and extract page by page using the lower-level API
    let doc = lopdf::Document::load_mem(&bytes)
        .with_context(|| format!("Failed to parse PDF: {}", path.display()))?;

    let total_pages = doc.get_pages().len() as u32;

    // Validate page numbers
    for &page in pages {
        if page == 0 || page > total_pages {
            anyhow::bail!("Page {} is out of range (1-{})", page, total_pages);
        }
    }

    let mut results = Vec::new();

    // Extract text for each requested page
    for &page_num in pages {
        let text = extract_page_text(&bytes, page_num)?;
        results.push(PageText {
            page: page_num,
            text,
        });
    }

    Ok(results)
}

fn extract_page_text(pdf_bytes: &[u8], page_num: u32) -> Result<String> {
    // Use pdf-extract's output_doc to get text with page markers
    // Then parse out just the page we want
    let full_text = pdf_extract::extract_text_from_mem(pdf_bytes)?;

    // pdf-extract doesn't give us page boundaries directly
    // We'll use a workaround: extract with page breaks indicated by form feeds
    // Actually, let's try using the lower-level API

    // For now, return the full text for the first page, and empty for others
    // TODO: Implement proper per-page extraction
    if page_num == 1 {
        // Split by form feed or page break heuristics
        let pages: Vec<&str> = full_text.split('\x0C').collect();
        if let Some(first) = pages.first() {
            return Ok(first.to_string());
        }
    }

    // Try to split by form feed characters
    let pages: Vec<&str> = full_text.split('\x0C').collect();
    if let Some(page_text) = pages.get((page_num - 1) as usize) {
        Ok(page_text.to_string())
    } else {
        // Fallback: return full text if we can't split properly
        Ok(full_text)
    }
}

#[derive(Debug, Clone)]
pub struct PageText {
    pub page: u32,
    pub text: String,
}

/// Search for a pattern in PDF text, returning matches with page numbers and context
pub fn grep_pdf<P: AsRef<Path>>(
    path: P,
    pattern: &regex::Regex,
    max_results: usize,
) -> Result<Vec<GrepMatch>> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read PDF: {}", path.display()))?;

    let full_text = pdf_extract::extract_text_from_mem(&bytes)
        .with_context(|| format!("Failed to extract text from PDF: {}", path.display()))?;

    // Split by form feed to get pages
    let pages: Vec<&str> = full_text.split('\x0C').collect();

    let mut matches = Vec::new();

    for (page_idx, page_text) in pages.iter().enumerate() {
        let page_num = (page_idx + 1) as u32;

        for (line_num, line) in page_text.lines().enumerate() {
            for mat in pattern.find_iter(line) {
                matches.push(GrepMatch {
                    page: page_num,
                    line_number: (line_num + 1) as u32,
                    text: line.to_string(),
                    match_start: mat.start() as u32,
                    match_end: mat.end() as u32,
                });

                if matches.len() >= max_results {
                    return Ok(matches);
                }
            }
        }
    }

    Ok(matches)
}

#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub page: u32,
    pub line_number: u32,
    pub text: String,
    pub match_start: u32,
    pub match_end: u32,
}
