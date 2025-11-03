// IDE: cargo-fuzz target
// Run with: cargo fuzz run fuzz_login_json
// Purpose: fuzz JSON -> LoginData deserialization -> Credentials construction
#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use techhub::domain::{UserName, UserPassword};

fuzz_target!(|data: &[u8]| {
    if let Ok(v) = serde_json::from_slice::<Value>(data) {
        let user_name = v
            .get("user_name")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        let password = v
            .get("password")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();

        // Test validation (what TryFrom does)
        let _ = UserName::parse(user_name);
        let _ = UserPassword::parse(password);
    }
});