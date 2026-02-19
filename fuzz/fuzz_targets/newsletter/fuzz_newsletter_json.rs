// IDE: cargo-fuzz target
// Run with: cargo fuzz run fuzz_newsletter_json
// Purpose: fuzz NewsLetterData JSON -> NewsletterContent::new(html, text)
#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use techhub::domain::NewsletterContent;

fuzz_target!(|data: &[u8]| {
    if let Ok(v) = serde_json::from_slice::<Value>(data) {
        // The ContentPayload has fields "html" and "text"
        let content = v.get("content").and_then(|c| c.as_object());
        let html = content
            .and_then(|c| c.get("html"))
            .and_then(|h| h.as_str())
            .unwrap_or("")
            .to_string();
        let text = content
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        // Call the domain constructor which runs NewsletterHtml::parse / NewsletterText::parse
        let _ = NewsletterContent::new(html, text);
    }
});