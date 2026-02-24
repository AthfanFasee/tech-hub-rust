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
