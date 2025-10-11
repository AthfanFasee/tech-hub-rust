use crate::domain::user_email::UserEmail;
use crate::domain::user_name::UserName;
use crate::domain::user_password::UserPassword;
use secrecy::Secret;
pub struct NewUser {
    pub email: UserEmail,
    pub name: UserName,
    pub password: UserPassword,
}
