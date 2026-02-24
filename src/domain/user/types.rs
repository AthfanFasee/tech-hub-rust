use crate::authentication::Credentials;
use crate::domain::{NewUser, UserName, UserPassword};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[derive(serde::Deserialize)]
pub struct LoginData {
    user_name: String,
    password: Secret<String>,
}

impl TryFrom<LoginData> for Credentials {
    type Error = String;

    fn try_from(payload: LoginData) -> Result<Self, Self::Error> {
        let user_name = UserName::parse(payload.user_name)?;
        let password = UserPassword::parse(payload.password.expose_secret().to_string())?;

        Ok(Credentials {
            user_name: user_name.as_ref().to_string(),
            password: password.into_secret(),
        })
    }
}

#[derive(Deserialize)]
pub struct UserData {
    email: String,
    user_name: String,
    password: Secret<String>,
}

// This is like saying - I know how to build myself `NewUser` from something else `UserData`
// Then Rust lets us use `.try_into` whenever there's a `UserData` - where it automatically tries converting it to a `NewUser`
impl TryFrom<UserData> for NewUser {
    type Error = String;

    fn try_from(payload: UserData) -> Result<Self, Self::Error> {
        NewUser::new(
            payload.email,
            payload.user_name,
            payload.password.expose_secret().to_string(),
        )
    }
}
