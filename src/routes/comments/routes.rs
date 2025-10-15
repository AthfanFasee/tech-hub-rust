use crate::authentication::reject_anonymous_users;
use crate::routes::{create_comment, delete_comment, show_comments_for_post};
use actix_web::middleware::from_fn;
use actix_web::web;

pub fn comment_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route("/get/post/{id}", web::get().to(show_comments_for_post))
        // Protected routes (require authentication)
        .service(
            web::scope("/me")
                .wrap(from_fn(reject_anonymous_users))
                .route("/create", web::post().to(create_comment))
                .route("/delete/{id}", web::delete().to(delete_comment)),
        );
}
