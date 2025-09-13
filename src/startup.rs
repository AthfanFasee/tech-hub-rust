use crate::configuration::{Configuration, DatabaseConfigs};
use crate::email_client::EmailClient;
use crate::routes::{add_user, health_check, publish_newsletter, user_confirm};
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;
use url::Url;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(config: Configuration) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&config.database);

        let sender_email = config.email_client.sender().expect("Invalid sender email");

        let timeout = config.email_client.timeout();
        let email_client = EmailClient::new(
            Url::parse(&config.email_client.base_url).expect("Invalid email client base URL"),
            sender_email,
            config.email_client.authorization_token,
            timeout,
        );

        let address = format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr()?.port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            config.application.base_url,
        )?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(config: &DatabaseConfigs) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(config.connect_options())
}

pub struct ApplicationBaseUrl(pub String);

pub fn run(
    tcp_listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/user/add", web::post().to(add_user))
            .route("/user/confirm", web::get().to(user_confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            // register the db connection as part of the application state
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(tcp_listener)?
    .run();

    Ok(server)
}
