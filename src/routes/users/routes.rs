use crate::authentication;
use crate::routes;
use actix_web::middleware;
use actix_web::web;

pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route("/login", web::post().to(routes::login))
        .route("/register", web::post().to(routes::register_user))
        .route(
            "/confirm/register",
            web::get().to(routes::confirm_user_activation),
        )
        .route("/confirm/subscribe", web::get().to(routes::subscribe_user))
        // Protected routes (require authentication)
        .service(
            web::scope("/me")
                .wrap(middleware::from_fn(authentication::reject_anonymous_users))
                .route("/reset-password", web::post().to(routes::change_password))
                .route("/logout", web::post().to(routes::log_out))
                .route(
                    "/email/subscribe",
                    web::get().to(routes::send_subscribe_email),
                )
                .route("/protected", web::get().to(routes::protected_endpoint)),
        );
}
