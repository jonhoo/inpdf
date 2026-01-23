mod cli;
mod commands;
mod mcp;
mod page_range;
mod pdf;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mcp => {
            mcp::run_server().await?;
        }
        Commands::Info { path } => {
            commands::info::run(&path)?;
        }
        Commands::Toc { path } => {
            commands::toc::run(&path)?;
        }
        Commands::Grep {
            pattern,
            path,
            ignore_case,
            max_results,
        } => {
            let options = commands::grep::GrepOptions {
                pattern,
                case_insensitive: ignore_case,
                max_results,
                ..Default::default()
            };
            commands::grep::run(&path, &options)?;
        }
        Commands::Extract {
            path,
            pages,
            output,
        } => {
            commands::extract::run(&path, &pages, &output)?;
        }
        Commands::Merge { inputs, output } => {
            let input_refs: Vec<_> = inputs.iter().collect();
            commands::merge::run(&input_refs, &output)?;
        }
        Commands::Split { path, output_dir } => {
            commands::split::run(&path, &output_dir)?;
        }
        Commands::PageLabels { path } => {
            let labels = pdf::page_labels::extract_page_labels(&path)?;
            for label in labels {
                println!("{}: {}", label.physical_page, label.logical_label);
            }
        }
        Commands::ReadPages { path, pages } => {
            let doc = pdf::PdfDocument::open(&path)?;
            let total = doc.page_count();
            let page_list = page_range::expand_page_ranges(&pages, total)?;
            let texts = pdf::text::extract_text_pages(&path, &page_list)?;

            for page_text in texts {
                println!("--- Page {} ---", page_text.page);
                println!("{}", page_text.text);
                println!();
            }
        }
    }

    Ok(())
}
