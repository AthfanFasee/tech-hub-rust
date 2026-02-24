// IDE: cargo-fuzz target
// Run with: cargo fuzz run fuzz_post_json
// Purpose: fuzz CreatePostPayload JSON -> Post::new(title, text, img)
#![no_main]

use libfuzzer_sys::fuzz_target;
use techhub::domain::Post;
use Value;

fuzz_target!(|data: &[u8]| {
    if let Ok(v) = serde_json::from_slice::<Value>(data) {
        let title = v.get("title").and_then(|s| s.as_str()).unwrap_or("").to_string();
        let text = v.get("text").and_then(|s| s.as_str()).unwrap_or("").to_string();
        let img = v.get("img").and_then(|s| s.as_str()).unwrap_or("").to_string();

        // Call the Post constructor which runs Title::parse / Text::parse / Img::parse
        // We intentionally drop the result; errors are expected for invalid inputs.
        let _ = Post::new(title, text, img);
    }
});
