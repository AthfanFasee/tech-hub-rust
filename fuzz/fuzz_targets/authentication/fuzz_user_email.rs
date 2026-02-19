// IDE: This is a cargo-fuzz target, not a normal module
// Run with: cargo fuzz run fuzz_user_email
// Purpose: Find email validation bypasses and crashes
// Focus: Security-critical input validation
#![no_main]

use libfuzzer_sys::fuzz_target;
use techhub::domain::UserEmail;
// Adjust to your crate name

fuzz_target!(|data: &[u8]| {
    // Convert raw bytes to string (fuzzer generates random bytes)
    if let Ok(s) = std::str::from_utf8(data) {
        let input = s.to_string();

        // We don't care about the result (Ok/Err), we're looking for:
        // 1. Panics (unwrap/expect failures)
        // 2. Infinite loops (timeouts)
        // 3. Memory issues (out-of-bounds, etc.)
        let _ = UserEmail::parse(input);
    }
});
