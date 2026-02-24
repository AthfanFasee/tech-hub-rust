use html5ever::driver;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

#[derive(Debug)]
pub struct NewsletterHtml(String);

impl NewsletterHtml {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err("Invalid newsletter HTML: cannot be empty.".to_string());
        }

        if trimmed.len() > 100_000 {
            return Err(
                "Invalid newsletter HTML: cannot be longer than 100,000 characters.".to_string(),
            );
        }

        // Validate that the string contains valid HTML
        if !Self::is_valid_html(trimmed) {
            return Err("Invalid newsletter HTML: must contain valid HTML tags.".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }

    fn is_valid_html(s: &str) -> bool {
        // First, check if the input string contains any HTML tag patterns
        // This prevents html5ever from auto-wrapping plain text
        if !Self::contains_html_tag_pattern(s) {
            return false;
        }

        // Parse the HTML document to ensure it's valid
        let dom = driver::parse_document(RcDom::default(), Default::default()).one(s);

        // Verify the parsed document has actual element nodes
        Self::has_element_nodes(&dom.document)
    }

    fn contains_html_tag_pattern(s: &str) -> bool {
        // Check for basic HTML tag pattern: <tagname...>
        // Must have both < and > and at least one character between them
        let mut in_tag = false;
        let mut has_tag_content = false;

        for c in s.chars() {
            if c == '<' {
                in_tag = true;
                has_tag_content = false;
            } else if c == '>' && in_tag {
                if has_tag_content {
                    return true;
                }
                in_tag = false;
            } else if in_tag && (c.is_alphanumeric() || c == '/' || c == '!') {
                has_tag_content = true;
            }
        }

        false
    }

    fn has_element_nodes(node: &Handle) -> bool {
        // Check children of the node
        for child in node.children.borrow().iter() {
            match child.data {
                NodeData::Element { .. } => {
                    // Found an element node
                    return true;
                }
                NodeData::Document => {
                    // Recursively check document children
                    if Self::has_element_nodes(child) {
                        return true;
                    }
                }
                _ => {
                    // Text nodes, comments, etc. - keep searching
                    if Self::has_element_nodes(child) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl AsRef<str> for NewsletterHtml {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NewsletterHtml {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
