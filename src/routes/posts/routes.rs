use actix_web::{middleware, web};

use crate::{authentication, routes};

pub fn post_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route("/get/all", web::get().to(routes::get_all_posts))
        .route("/get/{id}", web::get().to(routes::get_post))
        // Protected routes (require authentication)
        .service(
            web::scope("/me")
                .wrap(middleware::from_fn(authentication::reject_anonymous_users))
                .route("/create", web::post().to(routes::create_post))
                .route("/update/{id}", web::patch().to(routes::update_post))
                .route("/delete/{id}", web::delete().to(routes::delete_post))
                .route("/like/{id}", web::patch().to(routes::like_post))
                .route("/dislike/{id}", web::patch().to(routes::dislike_post)),
        );
}
