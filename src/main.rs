mod config;
mod external_service;
mod json_structure;
mod web;

use clap::Parser;
use config::Config;
use env_logger::{Builder as EnvLoggerBuilder, Target};
use external_service::spread_sheet;
use log;
use std::env;
use std::net::IpAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;

type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug, PartialEq)]
pub enum AppError {
    #[error("invalid spread sheet id:{0}")]
    ConfigError(#[from] config::ConfigError),

    #[error("google token managerr error ")]
    GoogleTokenManagerError,

    #[error("invalid token manager reference")]
    InvalidTokenManagerReferenceError,

    #[error("hyper error")]
    HyperError,
}

#[derive(Debug, Parser)]
pub struct Arg {
    #[clap(short, long, default_value = "127.0.0.1")]
    pub host: IpAddr,
    #[clap(short, long, default_value = "4000")]
    pub port: u16,

    #[clap(short, long)]
    pub service_account_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut conf = Config::from_env();
    let arg = Arg::parse();
    let Arg {
        host,
        port,
        service_account_file,
    } = arg;

    let mut log_builder = EnvLoggerBuilder::from_default_env();
    log_builder.target(Target::Stdout);
    if env::var("RUST_LOG").is_err() {
        log_builder.filter_level(log::LevelFilter::Info);
    }
    log_builder.init();

    if let Some(service_accont_file) = service_account_file {
        conf.set_service_account_file(service_accont_file)
    }

    let (token_refresh_finish_tx, token_refresh_finish_rx) = broadcast::channel(1);
    let token_manager = spread_sheet::token_manager_from_service_account_file(
        spread_sheet::scopes::SHEET_READ_ONLY,
        conf.service_account_file_as_path_buf()?, //TODO(tacogips) PathBuf to reference type
        token_refresh_finish_rx,
        None,
    )
    .await;

    let token_manager = match token_manager {
        Ok(tm) => tm,
        Err(e) => {
            log::error!("token manager generation failed: {:?}", e);
            return Err(AppError::GoogleTokenManagerError);
        }
    };

    let token_manager = Arc::new(token_manager);

    log::info!("service is listening at {}", port);
    if let Err(e) = web::run_server(conf, host, port, token_manager.clone()).await {
        log::error!("hyper error:{}", e);
        return Err(AppError::HyperError);
    }

    token_refresh_finish_tx.send(()).unwrap();

    let token_manager = match Arc::try_unwrap(token_manager) {
        Ok(token_manager) => token_manager,
        Err(_) => return Err(AppError::InvalidTokenManagerReferenceError),
    };
    if let Err(e) = token_manager.wait_until_refreshing_finished().await {
        log::error!("{}", e);
        return Err(AppError::GoogleTokenManagerError);
    }

    log::info!("service has shutdown ");
    Ok(())
}
