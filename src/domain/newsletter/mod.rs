mod newsletter_content;
mod newsletter_html;
mod newsletter_text;
mod newsletter_title;
mod types;

pub use newsletter_content::NewsletterContent;
pub use newsletter_html::NewsletterHtml;
pub use newsletter_text::NewsletterText;
pub use newsletter_title::NewsletterTitle;
pub use types::*;

#[derive(Debug)]
pub struct Newsletter {
    pub title: NewsletterTitle,
    pub content: NewsletterContent,
}

impl Newsletter {
    pub fn new(title: String, html: String, text: String) -> Result<Self, String> {
        Ok(Self {
            title: NewsletterTitle::parse(title)?,
            content: NewsletterContent::new(html, text)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Newsletter;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    // Example-based tests for Newsletter Title
    #[test]
    fn empty_title_is_rejected() {
        let result = Newsletter::new(
            "".into(),
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn long_title_is_rejected() {
        let long_title = "a".repeat(201);
        let result = Newsletter::new(
            long_title,
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn title_with_only_numbers_is_rejected() {
        let result = Newsletter::new(
            "12345".into(),
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn title_with_only_numbers_and_spaces_is_rejected() {
        let result = Newsletter::new(
            "123 456".into(),
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn title_with_numbers_and_letters_is_accepted() {
        let result = Newsletter::new(
            "Newsletter123".into(),
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn title_with_letters_and_numbers_is_accepted() {
        let result = Newsletter::new(
            "123Newsletter".into(),
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn title_at_max_length_is_accepted() {
        let title = "a".repeat(200);
        let result = Newsletter::new(
            title,
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    // Example-based tests for Newsletter HTML
    #[test]
    fn empty_html_is_rejected() {
        let result = Newsletter::new("Valid Title".into(), "".into(), "Valid text content".into());
        assert_err!(result);
    }

    #[test]
    fn whitespace_only_html_is_rejected() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "   \n\t   ".into(),
            "Valid text content".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn plain_text_without_html_tags_is_rejected() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "This is just plain text without any HTML tags".into(),
            "Valid text content".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn html_with_only_text_nodes_is_rejected() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "Just some text, no tags at all!".into(),
            "Valid text content".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn malformed_html_with_unclosed_tags_is_accepted() {
        // html5ever is a forgiving HTML5 parser. It automatically closes unclosed tags
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>Content without closing tag".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn simple_html_tag_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>Content</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn self_closing_html_tag_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<br />".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn html_with_attributes_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            r#"<a href="https://example.com">Link</a>"#.into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn html_with_nested_tags_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<div><p><strong>Bold text</strong></p></div>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn html_with_special_characters_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>Price: &euro;10 &amp; &lt;more&gt;</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn html_with_comments_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<!-- Comment --><p>Content</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn html_exceeding_max_length_is_rejected() {
        let long_html = format!("<p>{}</p>", "a".repeat(100_000));
        let result = Newsletter::new("Valid Title".into(), long_html, "Valid text content".into());
        assert_err!(result);
    }

    #[test]
    fn valid_html_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<html><body><h1>Newsletter</h1><p>Content here</p></body></html>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn html_at_max_length_is_accepted() {
        let content = "a".repeat(99_980);
        let html = format!("<p>{}</p>", content);
        let result = Newsletter::new("Valid Title".into(), html, "Valid text content".into());
        assert_ok!(result);
    }

    #[test]
    fn html_with_multiple_root_elements_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>First paragraph</p><p>Second paragraph</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn html_with_doctype_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<!DOCTYPE html><html><body><p>Content</p></body></html>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    // Example-based tests for Newsletter Text
    #[test]
    fn empty_text_is_rejected() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>Valid HTML content</p>".into(),
            "".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn whitespace_only_text_is_rejected() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>Valid HTML content</p>".into(),
            "   \n\t   ".into(),
        );
        assert_err!(result);
    }

    #[test]
    fn text_exceeding_max_length_is_rejected() {
        let long_text = "a".repeat(50_001);
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>Valid HTML content</p>".into(),
            long_text,
        );
        assert_err!(result);
    }

    #[test]
    fn valid_text_is_accepted() {
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>Valid HTML content</p>".into(),
            "This is the plain text version of the newsletter.".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn text_at_max_length_is_accepted() {
        let text = "a".repeat(50_000);
        let result = Newsletter::new(
            "Valid Title".into(),
            "<p>Valid HTML content</p>".into(),
            text,
        );
        assert_ok!(result);
    }

    // Combined validation tests
    #[test]
    fn valid_newsletter_with_all_fields_is_accepted() {
        let result = Newsletter::new(
            "Weekly Newsletter - January 2025".into(),
            "<html><body><h1>Hello Subscribers!</h1><p>This is our weekly update.</p></body></html>".into(),
            "Hello Subscribers! This is our weekly update.".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn newsletter_with_special_chars_in_title_is_accepted() {
        let result = Newsletter::new(
            "Newsletter: Updates & News!".into(),
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn newsletter_with_unicode_in_title_is_accepted() {
        let result = Newsletter::new(
            "📧 Newsletter 2025".into(),
            "<p>Valid HTML content</p>".into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    #[test]
    fn newsletter_with_multiline_html_is_accepted() {
        let html = r#"
            <html>
                <head><title>Newsletter</title></head>
                <body>
                    <h1>Welcome</h1>
                    <p>Content here</p>
                </body>
            </html>
        "#;
        let result = Newsletter::new(
            "Valid Title".into(),
            html.into(),
            "Valid text content".into(),
        );
        assert_ok!(result);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn valid_titles_with_valid_length_are_accepted(
            title in r"[a-zA-Z][a-zA-Z0-9 ]{0,199}",
        ) {
            let result = Newsletter::new(
                title,
                "<p>Valid HTML</p>".into(),
                "Valid text".into(),
            );
            prop_assert!(result.is_ok());
        }

        #[test]
        fn titles_longer_than_200_chars_are_rejected(
            title in r"[a-zA-Z0-9]{201,250}",
        ) {
            let result = Newsletter::new(
                title,
                "<p>Valid HTML</p>".into(),
                "Valid text".into(),
            );
            prop_assert!(result.is_err());
        }

        #[test]
        fn whitespace_only_titles_are_rejected(
            title in r"\s{1,50}",
        ) {
            let result = Newsletter::new(
                title,
                "<p>Valid HTML</p>".into(),
                "Valid text".into(),
            );
            prop_assert!(result.is_err());
        }

        #[test]
        fn numeric_only_titles_are_rejected(
            title in r"[0-9]{1,50}",
        ) {
            let result = Newsletter::new(
                title,
                "<p>Valid HTML</p>".into(),
                "Valid text".into(),
            );
            prop_assert!(result.is_err());
        }

        #[test]
        fn numeric_only_titles_with_spaces_are_rejected(
            num1 in r"[0-9]{1,20}",
            num2 in r"[0-9]{1,20}",
        ) {
            let title = format!("{} {}", num1, num2);
            let result = Newsletter::new(
                title,
                "<p>Valid HTML</p>".into(),
                "Valid text".into(),
            );
            prop_assert!(result.is_err());
        }

        #[test]
        fn html_content_within_limits_is_accepted(
            content in r"[a-zA-Z0-9<>/. ]{10,1000}",
        ) {
            let html = format!("<p>{}</p>", content);
            let result = Newsletter::new(
                "Valid Title".into(),
                html,
                "Valid text".into(),
            );
            prop_assert!(result.is_ok());
        }

        #[test]
        fn text_content_within_limits_is_accepted(
            content in r"[a-zA-Z0-9 .!?,]{10,1000}",
        ) {
            let result = Newsletter::new(
                "Valid Title".into(),
                "<p>Valid HTML</p>".into(),
                content,
            );
            prop_assert!(result.is_ok());
        }

        #[test]
        fn whitespace_only_html_content_is_rejected(
            html in r"\s{1,100}",
        ) {
            let result = Newsletter::new(
                "Valid Title".into(),
                html,
                "Valid text".into(),
            );
            prop_assert!(result.is_err());
        }

        #[test]
        fn whitespace_only_text_content_is_rejected(
            text in r"\s{1,100}",
        ) {
            let result = Newsletter::new(
                "Valid Title".into(),
                "<p>Valid HTML</p>".into(),
                text,
            );
            prop_assert!(result.is_err());
        }

        #[test]
        fn all_three_fields_must_be_valid_together(
            // Title must start with a letter to ensure it's not only numbers
            title in r"[a-zA-Z][a-zA-Z0-9 ]{0,199}",
            html_content in r"[a-zA-Z0-9<>/. ]{10,500}",
            text_content in r"[a-zA-Z0-9 .!?,]{10,500}",
        ) {
            let html = format!("<p>{}</p>", html_content);
            let result = Newsletter::new(title, html, text_content);
            // If all fields are valid individually, the newsletter should be valid
            prop_assert!(result.is_ok());
        }

        #[test]
        fn titles_with_mixed_alphanumeric_are_accepted(
            prefix in r"[a-zA-Z]{1,10}",
            number in r"[0-9]{1,5}",
            suffix in r"[a-zA-Z ]{0,20}",
        ) {
            let title = format!("{}{}{}", prefix, number, suffix);
            let result = Newsletter::new(
                title,
                "<p>Valid HTML</p>".into(),
                "Valid text".into(),
            );
            prop_assert!(result.is_ok());
        }

        #[test]
        fn very_long_html_exceeding_limit_is_rejected(
            // Generate content that will exceed 100,000 chars
            size in 100_001..110_000_usize,
        ) {
            let html = "a".repeat(size);
            let result = Newsletter::new(
                "Valid Title".into(),
                html,
                "Valid text".into(),
            );
            prop_assert!(result.is_err());
        }

        #[test]
        fn very_long_text_exceeding_limit_is_rejected(
            // Generate content that will exceed 50,000 chars
            size in 50_001..60_000_usize,
        ) {
            let text = "a".repeat(size);
            let result = Newsletter::new(
                "Valid Title".into(),
                "<p>Valid HTML</p>".into(),
                text,
            );
            prop_assert!(result.is_err());
        }
    }
}
