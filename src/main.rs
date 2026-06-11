use anyhow::Result;
use axum::{Json, response::Html, routing::get};
use axum_swagger_ui::swagger_ui;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::JobScheduler;
use tower_http::services::{ServeDir, ServeFile};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

mod calendar;
mod config;
mod error;
mod invoice;
mod openapi;
mod users;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "coworker_portal=info".parse().unwrap()),
        )
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let infra = config::InfraConfig::from_env()?;
    let users_config = users::Config::from_env()?;
    let invoice_config = invoice::Config::from_env()?;
    let calendar_config = calendar::Config::from_env()?;

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&infra.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("Migrations applied");

    let jwt = Arc::new(users::JwtService::new(
        &users_config.jwt_secret,
        users_config.jwt_expiry_hours,
    ));

    let users_state = users::State {
        db: db.clone(),
        jwt: jwt.clone(),
        config: Arc::new(users_config),
    };

    let unify_client: Arc<dyn invoice::unify::UnifyClient> = match invoice_config.unify.mode {
        invoice::config::UnifyMode::Mock => {
            tracing::info!("Unify: using mock client");
            Arc::new(invoice::unify::mock::MockUnifyClient)
        }
        invoice::config::UnifyMode::Real => {
            tracing::info!("Unify: connecting to {}", invoice_config.unify.base_url);
            Arc::new(invoice::unify::real::RealUnifyClient::new(&invoice_config.unify).await?)
        }
    };

    let superuser_session = invoice::django_pdf::acquire_django_session(
        &invoice_config.django_base_url,
        invoice_config.django_accept_invalid_certs,
        &invoice_config.django_superuser_username,
        &invoice_config.django_superuser_password,
    )
    .await
    .inspect_err(|e| tracing::warn!(error = %e, "Superuser Django session acquisition failed — invoice PDF will be unavailable"))
    .ok();

    let billing_directory: Arc<dyn invoice::ports::BillingDirectory> =
        Arc::new(users::adapters::PgBillingDirectory::new(db.clone()));

    let invoice_state = invoice::State {
        db: db.clone(),
        jwt: jwt.clone(),
        unify: unify_client,
        billing_directory,
        superuser_session: Arc::new(RwLock::new(superuser_session)),
        config: Arc::new(invoice_config),
    };

    let calendar_state = calendar::State {
        db: db.clone(),
        jwt: jwt.clone(),
        config: Arc::new(calendar_config),
    };

    let scheduler = JobScheduler::new().await?;
    invoice::tasks::register(&scheduler, invoice_state.clone()).await?;
    calendar::tasks::register(&scheduler, calendar_state.clone()).await?;
    scheduler.start().await?;

    let mut api_doc = openapi::ApiDoc::openapi();
    api_doc.merge(users::openapi::ApiDoc::openapi());
    api_doc.merge(invoice::openapi::ApiDoc::openapi());
    api_doc.merge(calendar::openapi::ApiDoc::openapi());

    let (router, api) = OpenApiRouter::with_openapi(api_doc)
        .nest("/api/auth", users::routes::auth_router().with_state(users_state.clone()))
        .nest("/api", users::routes::router().with_state(users_state))
        .nest("/api", invoice::routes::router().with_state(invoice_state))
        .nest("/api", calendar::routes::router().with_state(calendar_state))
        .split_for_parts();

    let app = router
        .route("/swagger", get(|| async { Html(swagger_ui("/api-docs/openapi.json"))}))
        .route("/api-docs/openapi.json", get(|| async move { Json(api) }))
        .fallback_service(ServeDir::new("public").fallback(ServeFile::new("public/index.html")));

    let addr: std::net::SocketAddr = infra.listen_addr.parse()?;

    match (infra.tls_cert_path.clone(), infra.tls_key_path.clone()) {
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
