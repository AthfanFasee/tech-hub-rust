use crate::authentication::reject_non_admin_users;
use crate::routes::publish_newsletter;
use actix_web::middleware::from_fn;
use actix_web::web;

pub fn admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/me")
            .wrap(from_fn(reject_non_admin_users))
            .route("/newsletters/publish", web::post().to(publish_newsletter)),
    );
}
