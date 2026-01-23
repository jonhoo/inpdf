use anyhow::{Context, Result};
use lopdf::Document;
use std::path::Path;

pub fn run<P: AsRef<Path>>(inputs: &[P], output: P) -> Result<()> {
    if inputs.is_empty() {
        anyhow::bail!("No input files specified");
    }

    if inputs.len() == 1 {
        // Just copy the single file
        std::fs::copy(&inputs[0], &output).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                inputs[0].as_ref().display(),
                output.as_ref().display()
            )
        })?;
        println!("Copied 1 file to {}", output.as_ref().display());
        return Ok(());
    }

    // Load first document as base
    let mut merged = Document::load(&inputs[0])
        .with_context(|| format!("Failed to load PDF: {}", inputs[0].as_ref().display()))?;

    let mut total_pages = merged.get_pages().len();

    // Merge remaining documents by copying their pages
    for input in &inputs[1..] {
        let doc = Document::load(input)
            .with_context(|| format!("Failed to load PDF: {}", input.as_ref().display()))?;

        let pages = doc.get_pages().len();
        total_pages += pages;

        // Get the page count and merge objects
        for (_, page_id) in doc.get_pages() {
            // Renumber object IDs to avoid conflicts
            let new_id = (merged.max_id + 1, 0);
            merged.max_id += 1;

            // Copy the page object
            if let Ok(page_obj) = doc.get_object(page_id) {
                merged.objects.insert(new_id, page_obj.clone());

                // Add to pages tree (simplified - may not work for all PDFs)
                if let Ok(catalog) = merged.catalog() {
                    if let Ok(pages_ref) = catalog.get(b"Pages") {
                        if let lopdf::Object::Reference(pages_id) = pages_ref {
                            if let Ok(pages_dict) = merged.get_dictionary_mut(*pages_id) {
                                if let Ok(lopdf::Object::Array(kids)) = pages_dict.get_mut(b"Kids")
                                {
                                    kids.push(lopdf::Object::Reference(new_id));
                                }
                                if let Ok(lopdf::Object::Integer(count)) =
                                    pages_dict.get_mut(b"Count")
                                {
                                    *count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    merged
        .save(&output)
        .with_context(|| format!("Failed to save merged PDF: {}", output.as_ref().display()))?;

    println!(
        "Merged {} files ({} pages) into {}",
        inputs.len(),
        total_pages,
        output.as_ref().display()
    );

    Ok(())
}
