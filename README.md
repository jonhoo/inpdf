# inpdf

A CLI tool and MCP server for searching, navigating, and extracting content from PDFs.

## What it does

```bash
# Search for text across a PDF
$ inpdf grep "authentication" spec.pdf
p12:L45: User authentication requires valid credentials
p12:L89: The authentication token expires after 24 hours
p45:L12: See Chapter 3 for authentication details

# Extract specific pages
$ inpdf extract manual.pdf "1-10,25,30-end" -o excerpt.pdf

# Read text from specific pages
$ inpdf read-pages textbook.pdf "5-7"
--- Page 5 ---
Chapter 2: Introduction to...

# Get document info
$ inpdf info report.pdf
File: report.pdf
Pages: 156
Title: Annual Report 2024
```

Page ranges support `1-5`, `10`, `15-end`, reverse order `5-1`, and combinations like `1-3,7,20-end`.

Run `inpdf --help` for all commands.

## Why use this?

**For AI/LLM workflows:** inpdf runs as an MCP server, letting Claude and other AI assistants directly search and read PDFs.

**For CLI users:** Fast regex search across PDFs and flexible page extraction without leaving the terminal.

## MCP Server Setup

### Claude Code (CLI)

```bash
claude mcp add --scope user inpdf /path/to/inpdf mcp
```

### Claude Desktop

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "inpdf": {
      "command": "/path/to/inpdf",
      "args": ["mcp"]
    }
  }
}
```

### Available MCP Tools

This exposes tools like `pdf_grep`, `pdf_read_pages`, `pdf_info`, `pdf_toc`, and `pdf_extract` to AI assistants.

## Installation

```bash
git clone https://github.com/youruser/inpdf
cd inpdf
cargo build --release
# Binary is at target/release/inpdf
```

Requires Rust 1.70+.

## Limitations

- Text extraction quality depends on how the PDF was created (scanned documents won't work well)
- Large PDFs may be slow for page extraction operations
