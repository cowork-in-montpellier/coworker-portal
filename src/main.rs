use anyhow::Result;
use axum::{Json, response::Html, routing::get};
use axum_swagger_ui::swagger_ui;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::{ServeDir, ServeFile};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

mod auth;
mod config;
mod domain;
mod error;
mod openapi;
mod routes;
mod tasks;
mod unify;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt: Arc<auth::jwt::JwtService>,
    pub unify: Arc<dyn unify::UnifyClient>,
    pub config: Arc<config::Config>,
    /// Cached superuser Django session for guest bill PDF proxy.
    /// Acquired at startup; refreshed on 403 responses.
    pub superuser_session: Arc<RwLock<Option<String>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "coworker_portal=debug,info".parse().unwrap()),
        )
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let config = config::Config::from_env()?;

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("Migrations applied");

    let unify_client: Arc<dyn unify::UnifyClient> = match config.unify.mode {
        config::UnifyMode::Mock => {
            tracing::info!("Unify: using mock client");
            Arc::new(unify::mock::MockUnifyClient)
        }
        config::UnifyMode::Real => {
            tracing::info!("Unify: connecting to {}", config.unify.base_url);
            Arc::new(unify::real::RealUnifyClient::new(&config.unify).await?)
        }
    };

    tracing::info!(smtp_user=&config.smtp.clone().unwrap().username, smtp_pass=&config.smtp.clone().unwrap().password, "Config");

    let superuser_session = auth::routes::acquire_django_session(
        &config.django_base_url,
        config.django_accept_invalid_certs,
        &config.django_superuser_username,
        &config.django_superuser_password,
    )
    .await
    .inspect_err(|e| tracing::warn!(error = %e, "Superuser Django session acquisition failed — guest PDF will be unavailable"))
    .ok();

    let state = AppState {
        db,
        jwt: Arc::new(auth::jwt::JwtService::new(
            &config.jwt_secret,
            config.jwt_expiry_hours,
        )),
        unify: unify_client,
        superuser_session: Arc::new(RwLock::new(superuser_session)),
        config: Arc::new(config.clone()),
    };

    tasks::start(
        state.clone(),
        &config.voucher_sync_cron,
        &config.monthly_usage_cron,
        config.google_calendar_ical_url.as_deref(),
        &config.google_calendar_sync_cron,
        config.google_calendar_room_id,
    ).await?;

    let (router, api) = OpenApiRouter::with_openapi(openapi::ApiDoc::openapi())
        .nest("/api/auth", auth::router())
        .nest("/api", routes::router())
        .split_for_parts();

    let app = router
        .route("/swagger", get(|| async { Html(swagger_ui("/api-docs/openapi.json"))}))
        .route("/api-docs/openapi.json", get(|| async move { Json(api) }))
        .fallback_service(ServeDir::new("public").fallback(ServeFile::new("public/index.html")))
        .with_state(state);

    let addr: std::net::SocketAddr = config.listen_addr.parse()?;

    match (config.tls_cert_path.clone(), config.tls_key_path.clone()) {
        (Some(cert), Some(key)) => {
            let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
            tracing::info!("Listening on https://{}", addr);
            axum_server::bind_rustls(addr, tls_config)
                .serve(app.into_make_service())
                .await?;
        }
        _ => {
            let listener = tokio::net::TcpListener::bind(addr).await?;
            tracing::info!("Listening on http://{}", addr);
            axum::serve(listener, app).await?;
        }
    }
    Ok(())
}
