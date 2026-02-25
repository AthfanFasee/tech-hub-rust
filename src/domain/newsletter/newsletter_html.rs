use std::fmt::{self, Display, Formatter};

use html5ever::{driver, tendril::TendrilSink};
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

impl Display for NewsletterHtml {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    use super::NewsletterHtml;

    // Example-based tests for Newsletter HTML
    #[test]
    fn empty_html_is_rejected() {
        let result = NewsletterHtml::parse("".into());
        assert_err!(result);
    }

    #[test]
    fn whitespace_only_html_is_rejected() {
        let result = NewsletterHtml::parse("   \n\t   ".into());
        assert_err!(result);
    }

    #[test]
    fn plain_text_without_html_tags_is_rejected() {
        let result = NewsletterHtml::parse("This is just plain text without any HTML tags".into());
        assert_err!(result);
    }

    #[test]
    fn html_with_only_text_nodes_is_rejected() {
        let result = NewsletterHtml::parse("Just some text, no tags at all!".into());
        assert_err!(result);
    }

    #[test]
    fn malformed_html_with_unclosed_tags_is_accepted() {
        // html5ever is a forgiving HTML5 parser. It automatically closes unclosed tags
        let result = NewsletterHtml::parse("<p>Content without closing tag".into());
        assert_ok!(result);
    }

    #[test]
    fn simple_html_tag_is_accepted() {
        let result = NewsletterHtml::parse("<p>Content</p>".into());
        assert_ok!(result);
    }

    #[test]
    fn self_closing_html_tag_is_accepted() {
        let result = NewsletterHtml::parse("<br />".into());
        assert_ok!(result);
    }

    #[test]
    fn html_with_attributes_is_accepted() {
        let result = NewsletterHtml::parse(r#"<a href="https://example.com">Link</a>"#.into());
        assert_ok!(result);
    }

    #[test]
    fn html_with_nested_tags_is_accepted() {
        let result = NewsletterHtml::parse("<div><p><strong>Bold text</strong></p></div>".into());
        assert_ok!(result);
    }

    #[test]
    fn html_with_special_characters_is_accepted() {
        let result = NewsletterHtml::parse("<p>Price: &euro;10 &amp; &lt;more&gt;</p>".into());
        assert_ok!(result);
    }

    #[test]
    fn html_with_comments_is_accepted() {
        let result = NewsletterHtml::parse("<!-- Comment --><p>Content</p>".into());
        assert_ok!(result);
    }

    #[test]
    fn html_exceeding_max_length_is_rejected() {
        let long_html = format!("<p>{}</p>", "a".repeat(100_000));
        let result = NewsletterHtml::parse(long_html);
        assert_err!(result);
    }

    #[test]
    fn valid_html_is_accepted() {
        let result = NewsletterHtml::parse(
            "<html><body><h1>Newsletter</h1><p>Content here</p></body></html>".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn html_at_max_length_is_accepted() {
        let content = "a".repeat(99_980);
        let html = format!("<p>{}</p>", content);
        let result = NewsletterHtml::parse(html);
        assert_ok!(result);
    }

    #[test]
    fn html_with_multiple_root_elements_is_accepted() {
        let result = NewsletterHtml::parse("<p>First paragraph</p><p>Second paragraph</p>".into());
        assert_ok!(result);
    }

    #[test]
    fn html_with_doctype_is_accepted() {
        let result =
            NewsletterHtml::parse("<!DOCTYPE html><html><body><p>Content</p></body></html>".into());
        assert_ok!(result);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn html_content_within_limits_is_accepted(
            content in r"[a-zA-Z0-9<>/. ]{10,1000}",
        ) {
            let html = format!("<p>{}</p>", content);
            let result = NewsletterHtml::parse(html);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn whitespace_only_html_content_is_rejected(
            html in r"\s{1,100}",
        ) {
            let result = NewsletterHtml::parse(html);
            prop_assert!(result.is_err());
        }

        #[test]
        fn very_long_html_exceeding_limit_is_rejected(
            // Generate content that will exceed 100,000 chars
            size in 100_001..110_000_usize,
        ) {
            let html = "a".repeat(size);
            let result = NewsletterHtml::parse(html);
            prop_assert!(result.is_err());
        }
    }
}
