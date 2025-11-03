// IDE: This is a cargo-fuzz target, not a normal module
// Run with: cargo fuzz run fuzz_user_email
// Purpose: Test grapheme counting edge cases
// Focus: Emoji, combining characters, zero-width joiners
#![no_main]

use libfuzzer_sys::fuzz_target;
use techhub::domain::UserPassword;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data).to_string();

    // Focus on cases that might break grapheme counting:
    // - Emoji with modifiers (👨‍👩‍👧‍👦 counts as 1 grapheme)
    // - Combining diacriticals (é = e + ´)
    // - Zero-width joiners/non-joiners
    let _ = UserPassword::parse(input);
});