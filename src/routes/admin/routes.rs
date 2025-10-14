use crate::authentication::reject_non_admin_users;
use crate::routes::{hard_delete_post, publish_newsletter};
use actix_web::middleware::from_fn;
use actix_web::web;

pub fn admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/me")
            .wrap(from_fn(reject_non_admin_users))
            .route("/newsletters/publish", web::post().to(publish_newsletter))
            .route("/post/delete/{id}", web::delete().to(hard_delete_post)),
    );
}
