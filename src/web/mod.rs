mod spread_sheet_handler;
mod spread_sheet_meta;
use crate::config::Config;
use axum::{
    error_handling::HandleErrorLayer,
    extract::Extension,
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, get_service},
    AddExtensionLayer, Json, Router,
};
use serde_json::json;

use crate::external_service::spread_sheet::TokenManager;
use futures::stream::StreamExt;
use signal_hook::consts::signal::*;
use signal_hook::iterator;
use signal_hook_tokio::{Signals, SignalsInfo};
use std::net::IpAddr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower::{BoxError, ServiceBuilder};
use tower_http::cors::{any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

const REQUEST_TIMEOUT_SEC: u64 = 60;
pub async fn run_server<HttpConnector>(
    config: Config,
    host: IpAddr,
    port: u16,
    token_manager: Arc<TokenManager<HttpConnector>>,
) -> Result<(), hyper::Error>
where
    HttpConnector: Clone + Send + Sync + 'static,
{
    let app = Router::new()
        .route("/meta", get(metadata))
        .route(
            "/sheet/:spread_sheet_id",
            get(spread_sheet_handler::get_spread_sheet_value::<HttpConnector>),
        )
        .route("/sheet_meta", get(spread_sheet_meta::get_spread_sheet_meta))
        .route(
            "/",
            get_service(ServeFile::new(format!(
                "{}/index.html",
                config.playground_file_dir
            )))
            .handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        )
        .nest(
            "/_next",
            get_service(ServeDir::new(format!(
                "{}/_next",
                config.playground_file_dir
            )))
            .handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        )
        .layer(
            CorsLayer::new()
                .allow_headers(any())
                .allow_origin(any())
                .allow_methods(vec![Method::GET]),
        )
        .layer(AddExtensionLayer::new(token_manager))
        .layer(AddExtensionLayer::new(config.clone()))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        ))
                    }
                }))
                .timeout(Duration::from_secs(REQUEST_TIMEOUT_SEC))
                .into_inner(),
        );

    let addr = SocketAddr::from((host, port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(server_shutdown_signal())
        .await
}

pub async fn metadata(Extension(config): Extension<Config>) -> impl IntoResponse {
    match config.service_account_data() {
        Err(e) => {
            log::error!("serviece account load error {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error_message":e.to_string()})),
            ))
        }
        Ok(sa) => Ok(Json(json!({"service_account":sa.client_email}))),
    }
}

pub async fn server_shutdown_signal() {
    let (handle, signals) = quit_signal_handler();

    let signals_task = tokio::spawn(handle_quit_signals(signals));
    signals_task.await.unwrap();
    handle.close();
    log::info!("quit singal received. starting graceful shutdown the web server")
}

pub fn quit_signal_handler() -> (iterator::backend::Handle, SignalsInfo) {
    let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT]).unwrap();
    (signals.handle(), signals)
}

pub async fn handle_quit_signals(signals: Signals) {
    let mut signals = signals.fuse();
    while let Some(signal) = signals.next().await {
        match signal {
            SIGHUP | SIGTERM | SIGINT | SIGQUIT => {
                // Shutdown the system;
                log::info!("shutdown signal has receipt");
                break;
            }
            _ => unreachable!(),
        }
    }
}
