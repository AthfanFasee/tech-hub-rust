use crate::authentication;
use crate::routes;
use actix_web::middleware;
use actix_web::web;

pub fn comment_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route(
            "/get/posts/{id}",
            web::get().to(routes::show_comments_for_post),
        )
        // Protected routes (require authentication)
        .service(
            web::scope("/me")
                .wrap(middleware::from_fn(authentication::reject_anonymous_users))
                .route("/create", web::post().to(routes::create_comment))
                .route("/delete/{id}", web::delete().to(routes::delete_comment)),
        );
}
