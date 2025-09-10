use crate::domain::user_email::UserEmail;
use crate::domain::user_name::UserName;
pub struct NewUser {
    pub email: UserEmail,
    pub name: UserName,
}
