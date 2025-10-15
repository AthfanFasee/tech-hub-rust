use crate::authentication::reject_anonymous_users;
use crate::routes::{
    create_post, delete_post, dislike_post, get_all_posts, get_post, like_post, update_post,
};
use actix_web::middleware::from_fn;
use actix_web::web;

pub fn post_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route("/get/all", web::get().to(get_all_posts))
        .route("/get/{id}", web::get().to(get_post))
        // Protected routes (require authentication)
        .service(
            web::scope("/me")
                .wrap(from_fn(reject_anonymous_users))
                .route("/create", web::post().to(create_post))
                .route("/update/{id}", web::patch().to(update_post))
                .route("/delete/{id}", web::delete().to(delete_post))
                .route("/like/{id}", web::patch().to(like_post))
                .route("/dislike/{id}", web::patch().to(dislike_post)),
        );
}
