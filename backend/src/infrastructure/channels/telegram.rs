use crate::domain::chat::{ChannelDelivery, ChannelHealth, InboundChannelMessage, OutboundChannelMessage};
use crate::domain::ports::ChannelConnectorPort;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};

const DEFAULT_TELEGRAM_API_BASE: &str = "https://api.telegram.org";
const TELEGRAM_SEND_MAX_ATTEMPTS: usize = 3;

enum SendFailure {
    Transient(String),
    Permanent(String),
}

#[derive(Clone)]
pub struct TelegramConnector {
    client: Client,
    bot_token: String,
    api_base_url: String,
}

impl TelegramConnector {
    pub fn new(bot_token: String, api_base_url: String) -> Self {
        Self {
            client: Client::new(),
            bot_token,
            api_base_url: normalize_api_base_url(&api_base_url),
        }
    }

    async fn send_text_once(
        &self,
        message: &OutboundChannelMessage,
        attempt: usize,
    ) -> std::result::Result<ChannelDelivery, SendFailure> {
        if self.api_base_url.starts_with("mock://") {
            if message.text.contains("[telegram-outage]") {
                return Err(SendFailure::Transient(
                    "mock telegram outage marker requested".to_string(),
                ));
            }
            if message.text.contains("[telegram-permanent]") {
                return Err(SendFailure::Permanent(
                    "mock telegram permanent error marker requested".to_string(),
                ));
            }
            let retry_failures = parse_retry_failures(&message.text);
            if attempt <= retry_failures {
                return Err(SendFailure::Transient(format!(
                    "mock telegram transient failure attempt {attempt}/{retry_failures}"
                )));
            }
            return Ok(ChannelDelivery {
                channel: "telegram".to_string(),
                external_message_id: Some("mock-telegram-message".to_string()),
            });
        }

        let url = format!("{}/bot{}/sendMessage", self.api_base_url, self.bot_token);
        let response = self
            .client
            .post(&url)
            .json(&json!({
                "chat_id": message.conversation_id,
                "text": message.text,
            }))
            .send()
            .await
            .map_err(|error| SendFailure::Transient(format!("failed to call telegram sendMessage: {url}: {error}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 || status.is_server_error() {
                return Err(SendFailure::Transient(format!(
                    "telegram api transient error {status}: {body}"
                )));
            }
            return Err(SendFailure::Permanent(format!(
                "telegram api error {status}: {body}"
            )));
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|error| {
                SendFailure::Transient(format!(
                    "failed to decode telegram sendMessage response: {error}"
                ))
            })?;
        let message_id = payload
            .get("result")
            .and_then(|value| value.get("message_id"))
            .and_then(|value| value.as_i64())
            .map(|value| value.to_string());

        Ok(ChannelDelivery {
            channel: "telegram".to_string(),
            external_message_id: message_id,
        })
    }
}

#[async_trait]
impl ChannelConnectorPort for TelegramConnector {
    fn channel(&self) -> &'static str {
        "telegram"
    }

    async fn start(&self) -> Result<()> {
        tracing::info!(channel = "telegram", api_base_url = %self.api_base_url, "telegram connector started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!(channel = "telegram", "telegram connector stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ChannelHealth> {
        Ok(ChannelHealth {
            channel: "telegram".to_string(),
            status: "ok".to_string(),
            detail: json!({
                "api_base_url": self.api_base_url,
                "mock_mode": self.api_base_url.starts_with("mock://"),
            }),
        })
    }

    async fn send(&self, message: OutboundChannelMessage) -> Result<ChannelDelivery> {
        let mut last_error = String::new();
        for attempt in 1..=TELEGRAM_SEND_MAX_ATTEMPTS {
            match self.send_text_once(&message, attempt).await {
                Ok(delivery) => {
                    if attempt > 1 {
                        tracing::info!(
                            channel = "telegram",
                            attempt,
                            "telegram send recovered after retry"
                        );
                    }
                    return Ok(delivery);
                }
                Err(SendFailure::Permanent(error)) => {
                    return Err(anyhow!(error));
                }
                Err(SendFailure::Transient(error)) => {
                    last_error = error;
                    if attempt < TELEGRAM_SEND_MAX_ATTEMPTS {
                        let backoff_ms = 100u64 * (1u64 << (attempt - 1));
                        tracing::warn!(
                            channel = "telegram",
                            attempt,
                            backoff_ms,
                            error = %last_error,
                            "telegram send transient error; retrying"
                        );
                        sleep(Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        }

        Err(anyhow!(
            "telegram send failed after {} attempts: {}",
            TELEGRAM_SEND_MAX_ATTEMPTS,
            last_error
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct TelegramWebhookUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
    pub edited_message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub text: Option<String>,
    pub chat: TelegramChat,
    pub from: Option<TelegramUser>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
struct TelegramGetUpdatesResponse {
    ok: bool,
    result: Vec<TelegramWebhookUpdate>,
}

impl TelegramWebhookUpdate {
    pub fn into_inbound_message(self) -> Option<InboundChannelMessage> {
        let message = self.message.or(self.edited_message)?;
        let text = message.text?.trim().to_string();
        if text.is_empty() {
            return None;
        }

        let user_id = message
            .from
            .map(|user| user.id.to_string())
            .unwrap_or_else(|| message.chat.id.to_string());
        let conversation_id = message.chat.id.to_string();

        Some(InboundChannelMessage {
            channel: "telegram".to_string(),
            user_id,
            conversation_id,
            message_id: Some(message.message_id.to_string()),
            text,
            metadata: json!({
                "update_id": self.update_id,
                "message_id": message.message_id,
            }),
        })
    }
}

pub fn normalize_api_base_url(value: &str) -> String {
    if value.trim().is_empty() {
        return DEFAULT_TELEGRAM_API_BASE.to_string();
    }
    value.trim().trim_end_matches('/').to_string()
}

pub async fn poll_updates_once(
    client: &Client,
    api_base_url: &str,
    bot_token: &str,
    offset: i64,
    timeout_secs: u64,
) -> Result<Vec<TelegramWebhookUpdate>> {
    let api_base_url = normalize_api_base_url(api_base_url);
    if api_base_url.starts_with("mock://") {
        return Ok(Vec::new());
    }

    let url = format!("{}/bot{}/getUpdates", api_base_url, bot_token);
    let response = client
        .get(&url)
        .query(&[
            ("offset", offset.to_string()),
            ("timeout", timeout_secs.to_string()),
        ])
        .send()
        .await
        .with_context(|| format!("failed to call telegram getUpdates: {url}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("telegram getUpdates error {status}: {body}"));
    }

    let payload: TelegramGetUpdatesResponse = response
        .json()
        .await
        .context("failed to decode telegram getUpdates response")?;
    if !payload.ok {
        return Err(anyhow!("telegram getUpdates returned ok=false"));
    }
    Ok(payload.result)
}

fn parse_retry_failures(text: &str) -> usize {
    let marker = "[telegram-retry:";
    let Some(start) = text.find(marker) else {
        return 0;
    };
    let tail = &text[start + marker.len()..];
    let Some(end) = tail.find(']') else {
        return 0;
    };
    tail[..end].trim().parse::<usize>().unwrap_or(0)
}
