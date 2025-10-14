use crate::authentication::reject_anonymous_users;
use crate::routes::{create_post, delete_post, protected_endpoint, update_post};
use actix_web::middleware::from_fn;
use actix_web::web;

pub fn post_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route("/get/all", web::get().to(protected_endpoint))
        .route("/get/{id}", web::get().to(protected_endpoint))
        // Protected routes (require authentication)
        .service(
            web::scope("/me")
                .wrap(from_fn(reject_anonymous_users))
                .route("/create", web::post().to(create_post))
                .route("/update/{id}", web::patch().to(update_post))
                .route("/delete/{id}", web::delete().to(delete_post))
                .route("/like/{id}", web::patch().to(protected_endpoint))
                .route("/dislike/{id}", web::patch().to(protected_endpoint)),
        );
}
