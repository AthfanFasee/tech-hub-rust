// IDE: cargo-fuzz target
// Run with: cargo fuzz run fuzz_register_json
// Purpose: fuzz JSON -> UserData -> TryFrom<UserData> for NewUser (exercises UserName, UserEmail, UserPassword)
#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use techhub::domain::{UserEmail, UserName, UserPassword};

fuzz_target!(|data: &[u8]| {
    if let Ok(v) = serde_json::from_slice::<Value>(data) {
        let user_name = v
            .get("user_name")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        let email = v
            .get("email")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        let password = v
            .get("password")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();

        // Call domain parsing functions that TryFrom uses internally
        let _ = UserName::parse(user_name);
        let _ = UserEmail::parse(email);
        let _ = UserPassword::parse(password);
    }
});
