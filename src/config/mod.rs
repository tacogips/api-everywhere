use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::path::PathBuf;
use thiserror::Error;

type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Error, Debug, PartialEq)]
pub enum ConfigError {
    #[error("no env vr {0}")]
    NoEnvVar(String),

    #[error("no service account file")]
    NoServiceAccountFile,

    #[error("service account load error")]
    ServiceAccountLoadError(String),
}

macro_rules! env_value {
    ($env_key:expr) => {
        env::var($env_key).map_err(|_| ConfigError::NoEnvVar($env_key.to_string()))
    };
}

#[derive(Clone)]
pub struct Config {
    pub service_account_file_path: Option<String>,
    pub playground_file_dir: String,
}

#[derive(Serialize, Deserialize)]
pub struct ServiceAccount {
    pub client_email: String,
}

impl Config {
    pub fn from_env() -> Self {
        let service_accocunt = env_value!("SERVICE_ACCONT_FILE");
        let service_account_file_path = match service_accocunt {
            Ok(service_account) => Some(service_account),
            Err(_) => env_value!("GOOGLE_APPLICATION_CREDENTIALS").ok(),
        };

        let playground_file_dir = env_value!("PLAYGROUND_DIR").unwrap_or_else(|_| {
            let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            p.push("src/playground_html");
            p.display().to_string()
        });

        Self {
            service_account_file_path,
            playground_file_dir,
        }
    }

    pub fn set_service_account_file(&mut self, file_path: String) {
        self.service_account_file_path = Some(file_path)
    }

    pub fn service_account_file_as_path_buf(&self) -> Result<PathBuf> {
        match self.service_account_file_path.as_ref() {
            Some(path) => {
                let mut pb = PathBuf::new();
                pb.push(path);
                Ok(pb)
            }
            None => Err(ConfigError::NoServiceAccountFile),
        }
    }

    pub fn service_account_data(&self) -> Result<ServiceAccount> {
        match &self.service_account_file_path {
            None => Err(ConfigError::NoServiceAccountFile),
            Some(file_path) => {
                let sa_file =
                    File::open(file_path).map_err(|_| ConfigError::NoServiceAccountFile)?;

                let sa_data: ServiceAccount = serde_json::from_reader(sa_file)
                    .map_err(|e| ConfigError::ServiceAccountLoadError(format!("{}", e)))?;
                Ok(sa_data)
            }
        }
    }
}
