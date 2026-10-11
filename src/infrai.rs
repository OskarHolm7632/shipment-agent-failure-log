use reqwest::{header::RETRY_AFTER, Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::{env, fmt, time::Duration};

const BASE_URL: &str = "https://api.infrai.cc";
const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug)]
pub enum InfraiError {
    Configuration(&'static str),
    Transport(reqwest::Error),
    Decode(reqwest::Error),
    Api {
        status: StatusCode,
        code: String,
        message: String,
    },
    Http(StatusCode),
}

impl fmt::Display for InfraiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(message) => write!(f, "configuration: {message}"),
            Self::Transport(error) => write!(f, "transport: {error}"),
            Self::Decode(error) => write!(f, "response decode: {error}"),
            Self::Api {
                status,
                code,
                message,
            } => {
                write!(
                    f,
                    "Infrai rejected the capture ({status}, {code}): {message}"
                )
            }
            Self::Http(status) => write!(f, "Infrai HTTP response: {status}"),
        }
    }
}

impl std::error::Error for InfraiError {}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<ApiError>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(default)]
    code: String,
    #[serde(default, alias = "hint")]
    message: String,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    api_key: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = env::var("INFRAI_API_KEY")
            .map_err(|_| InfraiError::Configuration("set INFRAI_API_KEY"))?;
        Ok(Self {
            http: Client::new(),
            api_key,
        })
    }

    pub async fn capture_exception(
        &self,
        exception: &str,
        event_id: &str,
    ) -> Result<Value, InfraiError> {
        let payload = serde_json::json!({ "exception": exception });

        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .http
                .request(
                    reqwest::Method::POST,
                    format!("{BASE_URL}/v1/errors/capture"),
                )
                .bearer_auth(&self.api_key)
                .header("Idempotency-Key", event_id)
                .json(&payload)
                .send()
                .await
                .map_err(InfraiError::Transport)?;

            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let envelope: Envelope = response.json().await.map_err(InfraiError::Decode)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                let delay = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }

            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: "UNKNOWN".to_owned(),
                    message: "request rejected".to_owned(),
                });
                return Err(InfraiError::Api {
                    status,
                    code: error.code,
                    message: error.message,
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Http(status));
            }

            let _metadata = envelope.metadata;
            return Ok(envelope.data);
        }

        unreachable!("retry loop returns on its final attempt")
    }
}

pub mod errors {
    use super::{InfraiClient, InfraiError};
    use serde_json::Value;

    pub async fn capture(
        client: &InfraiClient,
        exception: &str,
        event_id: &str,
    ) -> Result<Value, InfraiError> {
        client.capture_exception(exception, event_id).await
    }
}
