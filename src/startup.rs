use std::net::TcpListener;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use crate::routes::{health_check, add_user};
use tracing_actix_web::TracingLogger;
use crate::email_client::EmailClient;

pub fn run(
    tcp_listener: TcpListener, 
    db_pool: PgPool, 
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/user/add", web::post().to(add_user))
            // register the db connection as part of the application state
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
        .listen(tcp_listener)?
        .run();

    Ok(server)
}

// If req id comes from client side can capture it with this
// .wrap(
// TracingLogger::new().use_root_span_builder(|req| {
// let request_id = req
// .headers()
// .get("X-Request-Id")
// .and_then(|v| v.to_str().ok())
// .unwrap_or_else(|| Uuid::new_v4().to_string());
// 
// info_span!(
// "http_request",
// method = %req.method(),
// path = %req.path(),
// request_id = %request_id
// )
// })
// )