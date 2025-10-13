use crate::authentication::reject_anonymous_users;
use crate::routes::{
    change_password, confirm_user_activation, log_out, login, protected_endpoint, register_user,
    send_subscribe_email, subscribe_user,
};
use actix_web::middleware::from_fn;
use actix_web::web;

pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route("/login", web::post().to(login))
        .route("/register", web::post().to(register_user))
        .route("/confirm/register", web::get().to(confirm_user_activation))
        .route("/confirm/subscribe", web::get().to(subscribe_user))
        // Protected routes (require authentication)
        .service(
            web::scope("/me")
                .wrap(from_fn(reject_anonymous_users))
                .route("/reset-password", web::post().to(change_password))
                .route("/logout", web::post().to(log_out))
                .route("/email/subscribe", web::get().to(send_subscribe_email))
                .route("/protected", web::get().to(protected_endpoint)),
        );
}
