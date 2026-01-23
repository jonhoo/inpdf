use crate::pdf::PdfDocument;
use anyhow::Result;
use std::path::Path;

pub fn run<P: AsRef<Path>>(path: P) -> Result<()> {
    let doc = PdfDocument::open(&path)?;
    let info = doc.get_info();

    println!("File: {}", path.as_ref().display());
    println!("Pages: {}", info.page_count);

    if let Some(title) = &info.title {
        println!("Title: {}", title);
    }
    if let Some(author) = &info.author {
        println!("Author: {}", author);
    }
    if let Some(subject) = &info.subject {
        println!("Subject: {}", subject);
    }
    if let Some(keywords) = &info.keywords {
        println!("Keywords: {}", keywords);
    }
    if let Some(creator) = &info.creator {
        println!("Creator: {}", creator);
    }
    if let Some(producer) = &info.producer {
        println!("Producer: {}", producer);
    }
    if let Some(creation_date) = &info.creation_date {
        println!("Created: {}", format_pdf_date(creation_date));
    }
    if let Some(mod_date) = &info.mod_date {
        println!("Modified: {}", format_pdf_date(mod_date));
    }

    Ok(())
}

fn format_pdf_date(date: &str) -> String {
    // PDF date format: D:YYYYMMDDHHmmSSOHH'mm
    // Try to make it more readable
    if date.starts_with("D:") && date.len() >= 10 {
        let d = &date[2..];
        if d.len() >= 8 {
            let year = &d[0..4];
            let month = &d[4..6];
            let day = &d[6..8];
            let time = if d.len() >= 14 {
                format!(" {}:{}:{}", &d[8..10], &d[10..12], &d[12..14])
            } else {
                String::new()
            };
            return format!("{}-{}-{}{}", year, month, day, time);
        }
    }
    date.to_string()
}
