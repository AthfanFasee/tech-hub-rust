use actix_web::{middleware, web};

use crate::{authentication, routes};

pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route("/login", web::post().to(routes::login))
        .route("/register", web::post().to(routes::register_user))
        .route("/activate", web::get().to(routes::activate_user))
        .route("/subscribe", web::get().to(routes::subscribe_user))
        // Protected routes (require authentication)
        .service(
            web::scope("/me")
                .wrap(middleware::from_fn(authentication::reject_anonymous_users))
                .route("/change-password", web::post().to(routes::change_password))
                .route("/logout", web::post().to(routes::log_out))
                .route(
                    "/request-subscription",
                    web::get().to(routes::request_subscription),
                )
                .route("/protected", web::get().to(routes::protected_endpoint)),
        );
}
