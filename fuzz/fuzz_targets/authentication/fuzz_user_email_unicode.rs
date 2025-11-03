// IDE: This is a cargo-fuzz target, not a normal module
// Run with: cargo fuzz run fuzz_user_email
// Purpose: Specifically test Unicode edge cases in email validation
// Focus: IDN homograph attacks, combining characters, RTL text
#![no_main]

use libfuzzer_sys::fuzz_target;
use techhub::domain::UserEmail;

fuzz_target!(|data: &[u8]| {
    // This fuzzer doesn't require valid UTF-8, letting us test:
    // - Invalid UTF-8 sequences
    // - Partial multi-byte characters
    // - Byte order marks
    let input = String::from_utf8_lossy(data).to_string();

    let _ = UserEmail::parse(input);
});