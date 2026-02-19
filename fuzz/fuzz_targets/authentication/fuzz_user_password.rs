// IDE: This is a cargo-fuzz target, not a normal module
// Run with: cargo fuzz run fuzz_user_email
// Purpose: Find crashes in password validation
// Focus: Authentication-critical, must never panic
#![no_main]

use libfuzzer_sys::fuzz_target;
use techhub::domain::UserPassword;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let input = s.to_string();

        // Test password parsing with extreme inputs
        let _ = UserPassword::parse(input);
    }
});