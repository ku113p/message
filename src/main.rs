mod db;
mod router;

use axum::{Router, routing::get};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};

fn get_env_var(key: &str) -> Result<String, String> {
    std::env::var(key).map_err(|_| format!("{key} must be set"))
}

async fn ping_pong() -> &'static str {
    "pong"
}

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt().json()
        .with_max_level(Level::ERROR)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let db_url = get_env_var("POSTGRES_URL")?;
    let db = PgPool::connect(&db_url)
        .await
        .map_err(|_| "Failed to connect to database".to_string())?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed run migrations");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app_router = Router::new()
        .route("/ping", get(ping_pong))
        .nest("/contact", router::get_router(db.clone()).await)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let host = get_env_var("HOST")?;
    let port = get_env_var("PORT")?;
    let bind_address = format!("{}:{}", host, port);
    info!("Listening on {}", bind_address);
    let listener = tokio::net::TcpListener::bind(bind_address)
        .await
        .expect("Failed init listener");

    axum::serve(listener, app_router.into_make_service()).await.expect("Failed start serving");

    Ok(())
}
