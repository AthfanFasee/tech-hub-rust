use crate::domain::user_email::UserEmail;
use crate::domain::user_name::UserName;
use crate::domain::user_password::UserPassword;
pub struct NewUser {
    pub email: UserEmail,
    pub user_name: UserName,
    pub password: UserPassword,
}
