use crate::page_range::expand_page_ranges;
use crate::pdf::PdfDocument;
use anyhow::Result;
use std::path::Path;

pub fn run<P: AsRef<Path>, Q: AsRef<Path>>(input: P, pages: &str, output: Q) -> Result<()> {
    let doc = PdfDocument::open(&input)?;
    let total_pages = doc.page_count();

    let page_list = expand_page_ranges(pages, total_pages)?;

    if page_list.is_empty() {
        anyhow::bail!("No pages specified");
    }

    let mut new_doc = doc.extract_pages(&page_list)?;
    PdfDocument::save(&mut new_doc, &output)?;

    println!(
        "Extracted {} page(s) to {}",
        page_list.len(),
        output.as_ref().display()
    );

    Ok(())
}
