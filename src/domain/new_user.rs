use crate::domain::user_name::UserName;
use crate::domain::user_email::UserEmail;
pub struct NewUser {
    pub email: UserEmail,
    pub name: UserName,
}