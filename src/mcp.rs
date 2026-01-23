use anyhow::Result;
use regex::RegexBuilder;
use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt,
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
    #[tool(
        description = "Get PDF metadata including title, author, creator, producer, creation date, and page count"
    )]
    fn pdf_info(
        &self,
        Parameters(PathRequest { path }): Parameters<PathRequest>,
    ) -> Result<Json<PdfInfoResult>, String> {
        let doc = PdfDocument::open(&path).map_err(|e| e.to_string())?;
        let info = doc.get_info();
        Ok(Json(PdfInfoResult {
            path,
            page_count: info.page_count,
            title: info.title,
            author: info.author,
            creator: info.creator,
            producer: info.producer,
            creation_date: info.creation_date,
            subject: info.subject,
            keywords: info.keywords,
        }))
    }

    #[tool(
        description = "Get the table of contents (bookmarks/outlines) from a PDF as structured data"
    )]
    fn pdf_toc(
        &self,
        Parameters(PathRequest { path }): Parameters<PathRequest>,
    ) -> Result<Json<TocResult>, String> {
        let entries = extract_toc(&path).map_err(|e| e.to_string())?;
        let flat = flatten_toc(&entries);
        Ok(Json(TocResult {
            entries: flat
                .into_iter()
                .map(|e| TocEntryResult {
                    title: e.title,
                    page: e.page,
                    level: e.level,
                })
                .collect(),
        }))
    }

    #[tool(
        description = "Get the mapping between physical page numbers (1-indexed) and logical page labels"
    )]
    fn pdf_page_labels(
        &self,
        Parameters(PathRequest { path }): Parameters<PathRequest>,
    ) -> Result<Json<PageLabelsResult>, String> {
        let labels = extract_page_labels(&path).map_err(|e| e.to_string())?;
        Ok(Json(PageLabelsResult {
            labels: labels
                .into_iter()
                .map(|l| PageLabelResult {
                    physical_page: l.physical_page,
                    logical_label: l.logical_label,
                })
                .collect(),
        }))
    }

    #[tool(description = "Search for text in a PDF using a regular expression pattern")]
    fn pdf_grep(
        &self,
        Parameters(req): Parameters<PdfGrepRequest>,
    ) -> Result<Json<GrepResult>, String> {
        let regex = RegexBuilder::new(&req.pattern)
            .case_insensitive(req.case_insensitive)
            .build()
            .map_err(|e| format!("Invalid regex: {}", e))?;

        let matches =
            grep_pdf(&req.path, &regex, req.max_results as usize).map_err(|e| e.to_string())?;

        Ok(Json(GrepResult {
            matches: matches
                .into_iter()
                .map(|m| GrepMatchResult {
                    page: m.page,
                    line_number: m.line_number,
                    text: m.text,
                    match_start: m.match_start,
                    match_end: m.match_end,
                })
                .collect(),
        }))
    }

    #[tool(
        description = "Extract text content from specific pages of a PDF. Use page range syntax like '1-5,10,15-end'."
    )]
    fn pdf_read_pages(
        &self,
        Parameters(req): Parameters<PdfReadPagesRequest>,
    ) -> Result<Json<ReadPagesResult>, String> {
        let doc = PdfDocument::open(&req.path).map_err(|e| e.to_string())?;
        let total = doc.page_count();
        let page_list = expand_page_ranges(&req.pages, total).map_err(|e| e.to_string())?;
        let texts = extract_text_pages(&req.path, &page_list).map_err(|e| e.to_string())?;

        Ok(Json(ReadPagesResult {
            pages: texts
                .into_iter()
                .map(|t| PageTextResult {
                    page: t.page,
                    text: t.text,
                })
                .collect(),
        }))
    }

    #[tool(description = "Extract specific pages from a PDF and save them to a new file")]
    fn pdf_extract(
        &self,
        Parameters(req): Parameters<PdfExtractRequest>,
    ) -> Result<Json<ExtractResult>, String> {
        let doc = PdfDocument::open(&req.path).map_err(|e| e.to_string())?;
        let total = doc.page_count();
        let page_list = expand_page_ranges(&req.pages, total).map_err(|e| e.to_string())?;
        let page_count = page_list.len() as u32;

        let mut new_doc = doc.extract_pages(&page_list).map_err(|e| e.to_string())?;
        PdfDocument::save(&mut new_doc, &req.output).map_err(|e| e.to_string())?;

        Ok(Json(ExtractResult {
            output_path: req.output,
            page_count,
        }))
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
pub struct TocResult {
    pub entries: Vec<TocEntryResult>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PageLabelResult {
    pub physical_page: u32,
    pub logical_label: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PageLabelsResult {
    pub labels: Vec<PageLabelResult>,
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
pub struct GrepResult {
    pub matches: Vec<GrepMatchResult>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PageTextResult {
    pub page: u32,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReadPagesResult {
    pub pages: Vec<PageTextResult>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExtractResult {
    pub output_path: String,
    pub page_count: u32,
}

#[tool_handler]
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
    let service = server
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;

    service.waiting().await?;

    Ok(())
}
