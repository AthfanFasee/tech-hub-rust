use crate::domain::UserEmail;
use crate::domain::UserName;
use crate::domain::UserPassword;

pub struct NewUser {
    pub email: UserEmail,
    pub user_name: UserName,
    pub password: UserPassword,
}
