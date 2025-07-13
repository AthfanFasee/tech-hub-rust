use std::net::TcpListener;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use crate::routes::{health_check, subscribe};
pub fn run(tcp_listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new().route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // register the db connection as part of the application state
            .app_data(db_pool.clone())
    })
        .listen(tcp_listener)?
        .run();

    Ok(server)
}