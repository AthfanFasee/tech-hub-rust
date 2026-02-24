mod types;
mod user_email;
mod user_name;
mod user_password;

pub use types::*;
pub use user_email::UserEmail;
pub use user_name::UserName;
pub use user_password::UserPassword;

pub struct NewUser {
    pub email: UserEmail,
    pub user_name: UserName,
    pub password: UserPassword,
}

impl NewUser {
    pub fn new(email: String, user_name: String, password: String) -> Result<Self, String> {
        Ok(Self {
            email: UserEmail::parse(email)?,
            user_name: UserName::parse(user_name)?,
            password: UserPassword::parse(password)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::NewUser;
    use claims::assert_ok;
    use proptest::prelude::*;

    #[test]
    fn valid_user_is_accepted() {
        let result = NewUser::new(
            "user@example.com".into(),
            "John Doe".into(),
            "securepassword123".into(),
        );
        assert_ok!(result);
    }

    proptest! {
        #[test]
        fn all_three_fields_must_be_valid_together(
            username in r"[a-zA-Z][a-zA-Z0-9 ]{5,50}",
            domain in r"[a-z]{3,20}",
            password in r"[a-zA-Z0-9!@#$]{8,30}",
        ) {
            let email = format!("user@{}.com", domain);
            let result = NewUser::new(email, username, password);
            prop_assert!(result.is_ok());
        }
    }
}
