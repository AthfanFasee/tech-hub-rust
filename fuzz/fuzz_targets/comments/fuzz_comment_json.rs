// IDE: cargo-fuzz target
// Run with: cargo fuzz run fuzz_comment_json
// Purpose: fuzz CreateCommentPayload JSON -> Comment::new(text, post_id)
// Note: This is useful because post_id -> UUID parsing may reveal edge cases.
#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use techhub::domain::Comment;

fuzz_target!(|data: &[u8]| {
    if let Ok(v) = serde_json::from_slice::<Value>(data) {
        let text = v.get("text").and_then(|s| s.as_str()).unwrap_or("").to_string();
        let post_id = v.get("post_id").and_then(|s| s.as_str()).unwrap_or("").to_string();

        // This calls Uuid::parse_str inside Comment::new; useful to fuzz.
        let _ = Comment::new(text, post_id);
    }
});
