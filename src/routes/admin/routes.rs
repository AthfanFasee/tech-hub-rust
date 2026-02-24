use crate::authentication;
use crate::routes;
use actix_web::middleware;
use actix_web::web;

pub fn admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/me")
            .wrap(middleware::from_fn(authentication::reject_non_admin_users))
            .route(
                "/newsletters/publish",
                web::post().to(routes::publish_newsletter),
            )
            .route(
                "/posts/delete/{id}",
                web::delete().to(routes::hard_delete_post),
            ),
    );
}
