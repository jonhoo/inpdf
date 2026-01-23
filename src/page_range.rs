use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None,
    Right, // 90° clockwise (R)
    Down,  // 180° (D)
    Left,  // 90° counter-clockwise (L)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRange {
    pub start: PageRef,
    pub end: Option<PageRef>,
    pub rotation: Rotation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageRef {
    Number(u32),
    End,
}

impl PageRange {
    /// Parse a page range specification like "1-5", "9-6", "1-end", "5R"
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Err(anyhow!("Empty page range"));
        }

        // Check for rotation suffix (only if preceded by a digit, to avoid stripping from "end")
        let (range_part, rotation) = {
            let bytes = s.as_bytes();
            let len = bytes.len();
            if len >= 2 {
                let last = bytes[len - 1];
                let second_last = bytes[len - 2];
                if second_last.is_ascii_digit() {
                    match last {
                        b'R' | b'r' => (&s[..len - 1], Rotation::Right),
                        b'L' | b'l' => (&s[..len - 1], Rotation::Left),
                        b'D' | b'd' => (&s[..len - 1], Rotation::Down),
                        _ => (s, Rotation::None),
                    }
                } else {
                    (s, Rotation::None)
                }
            } else {
                (s, Rotation::None)
            }
        };

        // Parse the range
        if let Some(dash_pos) = range_part.find('-') {
            // Check if it's just a negative number (e.g., "-5" is invalid)
            if dash_pos == 0 {
                return Err(anyhow!("Invalid page range: {}", s));
            }

            let start_str = &range_part[..dash_pos];
            let end_str = &range_part[dash_pos + 1..];

            let start = parse_page_ref(start_str)?;
            let end = parse_page_ref(end_str)?;

            Ok(PageRange {
                start,
                end: Some(end),
                rotation,
            })
        } else {
            // Single page
            let page = parse_page_ref(range_part)?;
            Ok(PageRange {
                start: page,
                end: None,
                rotation,
            })
        }
    }

    /// Expand this range into a list of 1-based page numbers
    pub fn expand(&self, total_pages: u32) -> Result<Vec<u32>> {
        let start = match &self.start {
            PageRef::Number(n) => *n,
            PageRef::End => total_pages,
        };

        let end = match &self.end {
            Some(PageRef::Number(n)) => *n,
            Some(PageRef::End) => total_pages,
            None => start, // Single page
        };

        if start == 0 || end == 0 {
            return Err(anyhow!("Page numbers must be >= 1"));
        }

        if start > total_pages {
            return Err(anyhow!(
                "Start page {} exceeds total pages {}",
                start,
                total_pages
            ));
        }

        if end > total_pages {
            return Err(anyhow!(
                "End page {} exceeds total pages {}",
                end,
                total_pages
            ));
        }

        let pages: Vec<u32> = if start <= end {
            (start..=end).collect()
        } else {
            (end..=start).rev().collect()
        };

        Ok(pages)
    }
}

fn parse_page_ref(s: &str) -> Result<PageRef> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("end") {
        Ok(PageRef::End)
    } else {
        s.parse::<u32>()
            .map(PageRef::Number)
            .map_err(|_| anyhow!("Invalid page number: {}", s))
    }
}

/// Parse a comma-separated list of page ranges like "1-5,10,15-end"
pub fn parse_page_ranges(s: &str) -> Result<Vec<PageRange>> {
    s.split(',')
        .map(|part| PageRange::parse(part.trim()))
        .collect()
}

/// Expand a page range string into a list of 1-based page numbers
pub fn expand_page_ranges(s: &str, total_pages: u32) -> Result<Vec<u32>> {
    let ranges = parse_page_ranges(s)?;
    let mut pages = Vec::new();
    for range in ranges {
        pages.extend(range.expand(total_pages)?);
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_page() {
        let range = PageRange::parse("5").unwrap();
        assert_eq!(range.start, PageRef::Number(5));
        assert_eq!(range.end, None);
        assert_eq!(range.expand(10).unwrap(), vec![5]);
    }

    #[test]
    fn test_page_range() {
        let range = PageRange::parse("1-5").unwrap();
        assert_eq!(range.expand(10).unwrap(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_reverse_range() {
        let range = PageRange::parse("5-1").unwrap();
        assert_eq!(range.expand(10).unwrap(), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_end_keyword() {
        let range = PageRange::parse("5-end").unwrap();
        assert_eq!(range.expand(10).unwrap(), vec![5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_rotation() {
        let range = PageRange::parse("1-5R").unwrap();
        assert_eq!(range.rotation, Rotation::Right);
        assert_eq!(range.expand(10).unwrap(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_comma_separated() {
        let pages = expand_page_ranges("1-3,7,9-10", 10).unwrap();
        assert_eq!(pages, vec![1, 2, 3, 7, 9, 10]);
    }

    #[test]
    fn test_invalid_page_zero() {
        let range = PageRange::parse("0").unwrap();
        assert!(range.expand(10).is_err());
    }

    #[test]
    fn test_page_exceeds_total() {
        let range = PageRange::parse("15").unwrap();
        assert!(range.expand(10).is_err());
    }
}
