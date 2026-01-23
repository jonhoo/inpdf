use anyhow::Result;
use regex::RegexBuilder;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::page_range::expand_page_ranges;
use crate::pdf::page_labels::extract_page_labels;
use crate::pdf::text::{extract_text_pages, grep_pdf};
use crate::pdf::toc::{extract_toc, flatten_toc};
use crate::pdf::PdfDocument;

// Request structs for tools

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PathRequest {
    #[schemars(description = "Path to the PDF file")]
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfGrepRequest {
    #[schemars(description = "Path to the PDF file")]
    pub path: String,
    #[schemars(description = "Regular expression pattern to search for")]
    pub pattern: String,
    #[schemars(description = "Case insensitive search (default: false)")]
    #[serde(default)]
    pub case_insensitive: bool,
    #[schemars(description = "Maximum number of results (default: 100)")]
    #[serde(default = "default_max_results")]
    pub max_results: i32,
}

fn default_max_results() -> i32 {
    100
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfReadPagesRequest {
    #[schemars(description = "Path to the PDF file")]
    pub path: String,
    #[schemars(description = "Page ranges (e.g., '1-5,10,15-end')")]
    pub pages: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfExtractRequest {
    #[schemars(description = "Path to the source PDF file")]
    pub path: String,
    #[schemars(description = "Page ranges (e.g., '1-5,10,15-end')")]
    pub pages: String,
    #[schemars(description = "Output file path")]
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct PdfServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl PdfServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for PdfServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl PdfServer {
    #[tool(description = "Get PDF metadata including title, author, creator, producer, creation date, and page count")]
    fn pdf_info(&self, Parameters(PathRequest { path }): Parameters<PathRequest>) -> String {
        match PdfDocument::open(&path) {
            Ok(doc) => {
                let info = doc.get_info();
                let result = PdfInfoResult {
                    path,
                    page_count: info.page_count,
                    title: info.title,
                    author: info.author,
                    creator: info.creator,
                    producer: info.producer,
                    creation_date: info.creation_date,
                    subject: info.subject,
                    keywords: info.keywords,
                };
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Get the table of contents (bookmarks/outlines) from a PDF as structured data")]
    fn pdf_toc(&self, Parameters(PathRequest { path }): Parameters<PathRequest>) -> String {
        match extract_toc(&path) {
            Ok(entries) => {
                let flat = flatten_toc(&entries);
                let result: Vec<TocEntryResult> = flat
                    .into_iter()
                    .map(|e| TocEntryResult {
                        title: e.title,
                        page: e.page,
                        level: e.level,
                    })
                    .collect();
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Get the mapping between physical page numbers (1-indexed) and logical page labels")]
    fn pdf_page_labels(&self, Parameters(PathRequest { path }): Parameters<PathRequest>) -> String {
        match extract_page_labels(&path) {
            Ok(labels) => {
                let result: Vec<PageLabelResult> = labels
                    .into_iter()
                    .map(|l| PageLabelResult {
                        physical_page: l.physical_page,
                        logical_label: l.logical_label,
                    })
                    .collect();
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Search for text in a PDF using a regular expression pattern")]
    fn pdf_grep(&self, Parameters(req): Parameters<PdfGrepRequest>) -> String {
        let regex = match RegexBuilder::new(&req.pattern)
            .case_insensitive(req.case_insensitive)
            .build()
        {
            Ok(r) => r,
            Err(e) => return format!("Error: Invalid regex: {}", e),
        };

        match grep_pdf(&req.path, &regex, req.max_results as usize) {
            Ok(matches) => {
                let result: Vec<GrepMatchResult> = matches
                    .into_iter()
                    .map(|m| GrepMatchResult {
                        page: m.page,
                        line_number: m.line_number,
                        text: m.text,
                        match_start: m.match_start,
                        match_end: m.match_end,
                    })
                    .collect();
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Extract text content from specific pages of a PDF. Use page range syntax like '1-5,10,15-end'.")]
    fn pdf_read_pages(&self, Parameters(req): Parameters<PdfReadPagesRequest>) -> String {
        let doc = match PdfDocument::open(&req.path) {
            Ok(d) => d,
            Err(e) => return format!("Error: {}", e),
        };
        let total = doc.page_count();

        let page_list = match expand_page_ranges(&req.pages, total) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        match extract_text_pages(&req.path, &page_list) {
            Ok(texts) => {
                let result: Vec<PageTextResult> = texts
                    .into_iter()
                    .map(|t| PageTextResult {
                        page: t.page,
                        text: t.text,
                    })
                    .collect();
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e))
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Extract specific pages from a PDF and save them to a new file")]
    fn pdf_extract(&self, Parameters(req): Parameters<PdfExtractRequest>) -> String {
        let doc = match PdfDocument::open(&req.path) {
            Ok(d) => d,
            Err(e) => return format!("Error: {}", e),
        };
        let total = doc.page_count();

        let page_list = match expand_page_ranges(&req.pages, total) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };
        let page_count = page_list.len() as u32;

        let mut new_doc = match doc.extract_pages(&page_list) {
            Ok(d) => d,
            Err(e) => return format!("Error: {}", e),
        };

        if let Err(e) = PdfDocument::save(&mut new_doc, &req.output) {
            return format!("Error: {}", e);
        }

        let result = ExtractResult {
            output_path: req.output,
            page_count,
        };
        serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e))
    }
}

// Result types for MCP tools

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PdfInfoResult {
    pub path: String,
    pub page_count: u32,
    pub title: Option<String>,
    pub author: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TocEntryResult {
    pub title: String,
    pub page: Option<u32>,
    pub level: u32,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PageLabelResult {
    pub physical_page: u32,
    pub logical_label: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GrepMatchResult {
    pub page: u32,
    pub line_number: u32,
    pub text: String,
    pub match_start: u32,
    pub match_end: u32,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PageTextResult {
    pub page: u32,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExtractResult {
    pub output_path: String,
    pub page_count: u32,
}

impl ServerHandler for PdfServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "PDF navigation and manipulation tools. Use pdf_info to get document metadata, \
                 pdf_toc for table of contents, pdf_grep to search text, pdf_read_pages to extract \
                 text from specific pages, and pdf_extract to create new PDFs from page ranges."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub async fn run_server() -> Result<()> {
    let server = PdfServer::new();

    // Serve using stdin/stdout as a tuple
    let service = server.serve((tokio::io::stdin(), tokio::io::stdout())).await?;

    service.waiting().await?;

    Ok(())
}
