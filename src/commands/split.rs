use crate::pdf::PdfDocument;
use anyhow::{Context, Result};
use std::path::Path;

pub fn run<P: AsRef<Path>, Q: AsRef<Path>>(input: P, output_dir: Q) -> Result<()> {
    let input = input.as_ref();
    let output_dir = output_dir.as_ref();

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;

    let doc = PdfDocument::open(input)?;
    let total_pages = doc.page_count();

    // Get the base name of the input file
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page");

    for page_num in 1..=total_pages {
        let output_path = output_dir.join(format!("{}_{:04}.pdf", stem, page_num));

        let mut new_doc = doc.extract_pages(&[page_num])?;
        PdfDocument::save(&mut new_doc, &output_path)?;
    }

    println!(
        "Split {} pages into {}",
        total_pages,
        output_dir.display()
    );

    Ok(())
}
