use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "inpdf")]
#[command(about = "PDF navigation and manipulation tool with MCP server support")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run as MCP server (primary mode)
    Mcp,

    /// Display PDF metadata
    Info {
        /// PDF file to inspect
        path: PathBuf,
    },

    /// Print table of contents / bookmarks
    Toc {
        /// PDF file to inspect
        path: PathBuf,
    },

    /// Search text in PDF with regex
    Grep {
        /// Regular expression pattern to search for
        pattern: String,

        /// PDF file to search
        path: PathBuf,

        /// Case insensitive search
        #[arg(short, long)]
        ignore_case: bool,

        /// Maximum number of results
        #[arg(short, long, default_value = "100")]
        max_results: usize,
    },

    /// Extract page ranges to a new PDF
    #[command(alias = "cat")]
    Extract {
        /// PDF file to extract from
        path: PathBuf,

        /// Page ranges (e.g., "1-5,10,15-end")
        pages: String,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Combine multiple PDFs into one
    Merge {
        /// PDF files to merge
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Split PDF into individual pages
    #[command(alias = "burst")]
    Split {
        /// PDF file to split
        path: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output_dir: PathBuf,
    },

    /// Show page label mapping (logical vs physical page numbers)
    PageLabels {
        /// PDF file to inspect
        path: PathBuf,
    },

    /// Extract text from specific pages
    ReadPages {
        /// PDF file to read
        path: PathBuf,

        /// Page ranges (e.g., "1-5,10")
        pages: String,
    },
}
