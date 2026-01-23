use crate::pdf::toc::{extract_toc, flatten_toc};
use anyhow::Result;
use std::path::Path;

pub fn run<P: AsRef<Path>>(path: P) -> Result<()> {
    let entries = extract_toc(&path)?;

    if entries.is_empty() {
        println!("No table of contents found.");
        return Ok(());
    }

    let flat = flatten_toc(&entries);

    for entry in flat {
        let indent = "  ".repeat(entry.level as usize);
        let page_str = entry
            .page
            .map(|p| format!(" (p. {})", p))
            .unwrap_or_default();
        println!("{}{}{}", indent, entry.title, page_str);
    }

    Ok(())
}
