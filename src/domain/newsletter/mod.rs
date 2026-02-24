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
    use claims::assert_ok;
    use proptest::prelude::*;

    #[test]
    fn valid_newsletter_with_all_fields_is_accepted() {
        let result = Newsletter::new(
            "Weekly Newsletter - January 2025".into(),
            "<html><body><h1>Hello Subscribers!</h1><p>This is our weekly update.</p></body></html>".into(),
            "Hello Subscribers! This is our weekly update.".into(),
        );
        assert_ok!(result);
    }

    proptest! {
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
    }
}
