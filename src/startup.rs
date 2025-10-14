use crate::configuration::{Configuration, DatabaseConfigs};
use crate::email_client::EmailClient;
use crate::routes::{admin_routes, health_check, post_routes, user_routes};
use actix_session::SessionMiddleware;
use actix_session::storage::RedisSessionStore;
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use anyhow::Context;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(config: Configuration) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&config.database);

        let email_client = config.email_client.client();

        let address = format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(address)
            .with_context(|| "Failed to bind TCP listener for application")?;
        let port = listener
            .local_addr()
            .with_context(|| "Failed to read local address of TCP listener")?
            .port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            config.application.base_url,
            config.application.hmac_secret,
            config.application.redis_uri,
        )
        .await
        .context("Failed to run Actix web server")?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), anyhow::Error> {
        // run returns a Server type, which implements Future trait
        self.server.await.context("Server stopped with an error")
    }
}

pub fn get_connection_pool(config: &DatabaseConfigs) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(config.connect_options())
}

pub struct ApplicationBaseUrl(pub String);

async fn run(
    tcp_listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let secret_key = actix_web::cookie::Key::from(hmac_secret.expose_secret().as_bytes());

    let redis_store = RedisSessionStore::new(redis_uri.expose_secret())
        .await
        .context("Failed to connect to Redis session store")?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .configure(configure_routes)
            // register the db connection as part of the application state
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(tcp_listener)
    .with_context(|| "Failed to bind Actix server to TCP listener")?
    .run();

    Ok(server)
}

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health_check", web::get().to(health_check))
        .service(
            web::scope("/v1")
                .service(web::scope("/user").configure(user_routes))
                .service(web::scope("/admin").configure(admin_routes))
                .service(web::scope("/post").configure(post_routes)),
        );
}
