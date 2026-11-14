use arc_swap::ArcSwap;
use chrono::{Duration, Local};
use hyper;
use log;
use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::task::{JoinError, JoinHandle};
use tokio::time::timeout;
use yup_oauth2::{
    self as oauth,
    authenticator::{Authenticator, DefaultHyperClient, HyperClientBuilder},
    AccessToken, Error as OauthError,
};

type Result<T> = std::result::Result<T, GoogleTokenManagerError>;

#[derive(Error, Debug)]
pub enum GoogleTokenManagerError {
    #[error("oauth error:{0}")]
    OauthError(#[from] OauthError),

    #[error("failed to load service account file:{0}, {1}")]
    ServiceAccountFileLoadError(PathBuf, std::io::Error),

    #[error("invalid service account file:{0}, {1}")]
    InvalidServiceAccountFileError(PathBuf, std::io::Error),

    #[error("async task join error:{0}")]
    JoinError(#[from] JoinError),
}

static TOKEN_BUFFER_DURATION_TO_EXPIRE: OnceCell<Duration> = OnceCell::new();

fn get_token_buffer_duraiton_to_expire() -> &'static Duration {
    TOKEN_BUFFER_DURATION_TO_EXPIRE.get_or_init(|| Duration::minutes(2))
}

#[allow(dead_code)]
pub struct TokenManager<HttpConnector> {
    authenticator: Arc<Authenticator<HttpConnector>>,
    scopes: &'static [&'static str],
    inner_current_token: Arc<ArcSwap<AccessToken>>,
    token_refreshing_loop_jh: JoinHandle<()>,
}

impl<HttpConnector> TokenManager<HttpConnector>
where
    HttpConnector: hyper::client::connect::Connect + Clone + Send + Sync + 'static,
{
    pub async fn start(
        authenticator: Authenticator<HttpConnector>,
        scopes: &'static [&'static str],
        stop_refreshing_notifyer_rx: broadcast::Receiver<()>,

        token_refresh_period: Option<Duration>,
    ) -> Result<Self> {
        let access_token = authenticator.token(scopes.as_ref()).await?;
        let current_token = Arc::new(ArcSwap::from(Arc::new(access_token)));

        let authenticator = Arc::new(authenticator);

        let token_refreshing_loop_jh = Self::periodically_refreshing_token(
            authenticator.clone(),
            current_token.clone(),
            scopes,
            stop_refreshing_notifyer_rx,
            token_refresh_period,
        )
        .await;

        let result = Self {
            authenticator,
            scopes,
            inner_current_token: current_token,
            token_refreshing_loop_jh,
        };
        Ok(result)
    }

    async fn periodically_refreshing_token(
        authenticator: Arc<Authenticator<HttpConnector>>,
        shared_token: Arc<ArcSwap<AccessToken>>,
        scopes: &'static [&'static str],
        mut stop_refreshing_notifyer_rx: broadcast::Receiver<()>,
        token_refresh_period: Option<Duration>,
    ) -> JoinHandle<()> {
        let shared_token_current = shared_token.clone();

        //stop_refreshing_notifyer_rx
        // TODO(tacogips) Is that OK that tokio::spawn contains loop in it.
        let refresh_token_loop_jh = tokio::spawn(async move {
            let refresh_period = token_refresh_period
                .map(|p| p.to_std().unwrap())
                .unwrap_or_else(|| std::time::Duration::from_secs(30));
            loop {
                let has_stop_notified =
                    timeout(refresh_period, stop_refreshing_notifyer_rx.recv()).await;

                if has_stop_notified.is_ok() {
                    log::info!("exiting from auth token refreshing loop");
                    break;
                }

                let current_token = shared_token_current.load();
                let need_refresh = (**current_token)
                    .expiration_time()
                    .map(|expiration_time| {
                        expiration_time - *get_token_buffer_duraiton_to_expire() <= Local::now()
                    })
                    .unwrap_or(false);

                if need_refresh {
                    let new_token = Self::get_new_token(&authenticator, &scopes).await;
                    match new_token {
                        Ok(access_token) => shared_token.store(Arc::new(access_token)),
                        Err(e) => {
                            log::error!("failed to refresh token :{}", e);
                        }
                    }
                }
            }

            log::info!("exit from refreshing token loop")
        });
        refresh_token_loop_jh
    }

    #[allow(dead_code)]
    pub fn authenticator(&self) -> Arc<Authenticator<HttpConnector>> {
        Arc::clone(&self.authenticator)
    }

    #[allow(dead_code)]
    pub async fn force_refresh_token(&mut self) -> Result<()> {
        let new_token = Self::get_new_token(&self.authenticator, &self.scopes).await;
        match new_token {
            Ok(access_token) => {
                self.current_token().store(Arc::new(access_token));
                Ok(())
            }
            Err(e) => {
                log::error!("failed to refresh token :{}", e);
                return Err(e);
            }
        }
    }

    async fn get_new_token(
        authenticator: &Authenticator<HttpConnector>,
        scopes: &'static [&'static str],
    ) -> Result<AccessToken> {
        let new_token = authenticator.force_refreshed_token(scopes).await?;
        Ok(new_token)
    }

    pub async fn wait_until_refreshing_finished(self: Self) -> Result<()> {
        self.token_refreshing_loop_jh.await?;
        Ok(())
    }
}

impl<ANY> TokenManager<ANY> {
    pub fn current_token(&self) -> Arc<ArcSwap<AccessToken>> {
        Arc::clone(&self.inner_current_token)
    }
}

pub async fn token_manager_from_service_account_file(
    scopes: &'static [&'static str],
    service_account_cred_file: PathBuf, //TODO(tacogips) PathBuf to reference type
    stop_refreshing_notifyer_rx: broadcast::Receiver<()>,
    token_refresh_period: Option<Duration>,
) -> Result<TokenManager<<DefaultHyperClient as HyperClientBuilder>::Connector>> {
    let sa_key = oauth::read_service_account_key(&service_account_cred_file)
        .await
        .map_err(|e| {
            GoogleTokenManagerError::ServiceAccountFileLoadError(
                service_account_cred_file.clone(),
                e,
            )
        })?;

    let authenticator = oauth::ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await
        .map_err(|e| {
            GoogleTokenManagerError::InvalidServiceAccountFileError(service_account_cred_file, e)
        })?;

    TokenManager::start(
        authenticator,
        scopes,
        stop_refreshing_notifyer_rx,
        token_refresh_period,
    )
    .await
}

#[cfg(all(test, feature = "test-using-sa"))]
mod test {
    use super::super::scopes;
    use super::super::test::load_test_sa_file_path;
    use super::token_manager_from_service_account_file;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn load_token_manager_test() {
        let (_, rx) = broadcast::channel(1);
        let token_manager = token_manager_from_service_account_file(
            scopes::SHEET_READ_ONLY,
            load_test_sa_file_path(),
            rx,
            None,
        )
        .await;
        assert!(token_manager.is_ok());
        let token_manager = token_manager.unwrap();
        let token = token_manager.current_token();
        assert_ne!("", token.load().as_str());
    }
}
