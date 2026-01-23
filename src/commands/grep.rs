use crate::pdf::text::grep_pdf;
use anyhow::Result;
use regex::RegexBuilder;
use std::path::Path;

pub struct GrepOptions {
    pub pattern: String,
    pub case_insensitive: bool,
    pub max_results: usize,
    pub context_chars: usize,
}

impl Default for GrepOptions {
    fn default() -> Self {
        GrepOptions {
            pattern: String::new(),
            case_insensitive: false,
            max_results: 100,
            context_chars: 60,
        }
    }
}

pub fn run<P: AsRef<Path>>(path: P, options: &GrepOptions) -> Result<()> {
    let regex = RegexBuilder::new(&options.pattern)
        .case_insensitive(options.case_insensitive)
        .build()?;

    let matches = grep_pdf(&path, &regex, options.max_results)?;

    if matches.is_empty() {
        println!("No matches found.");
        return Ok(());
    }

    for m in &matches {
        // Truncate long lines for display
        let display_text = if m.text.len() > options.context_chars * 2 {
            let start = m.match_start as usize;
            let end = m.match_end as usize;

            // Show context around the match
            let ctx_start = start.saturating_sub(options.context_chars);
            let ctx_end = (end + options.context_chars).min(m.text.len());

            let mut display = String::new();
            if ctx_start > 0 {
                display.push_str("...");
            }
            display.push_str(&m.text[ctx_start..ctx_end]);
            if ctx_end < m.text.len() {
                display.push_str("...");
            }
            display
        } else {
            m.text.clone()
        };

        println!("p{}:L{}: {}", m.page, m.line_number, display_text.trim());
    }

    println!("\n{} match(es) found.", matches.len());

    Ok(())
}
