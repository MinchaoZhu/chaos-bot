pub mod telegram;

use crate::domain::chat::{ChannelDelivery, ChannelHealth, OutboundChannelMessage};
use crate::domain::ports::{ChannelConnectorPort, ChannelDispatcherPort};
use crate::infrastructure::channels::telegram::TelegramConnector;
use crate::infrastructure::config::AppConfig;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ChannelDispatcherRegistry {
    connectors: HashMap<String, Arc<dyn ChannelConnectorPort>>,
}

impl ChannelDispatcherRegistry {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    pub fn register(&mut self, connector: Arc<dyn ChannelConnectorPort>) {
        self.connectors
            .insert(connector.channel().to_string(), connector);
    }
}

#[async_trait]
impl ChannelDispatcherPort for ChannelDispatcherRegistry {
    async fn dispatch(&self, message: OutboundChannelMessage) -> Result<ChannelDelivery> {
        let channel = message.channel.clone();
        let connector = self
            .connectors
            .get(&channel)
            .ok_or_else(|| anyhow!("no connector registered for channel: {channel}"))?;
        connector.send(message).await
    }

    async fn start_all(&self) -> Result<()> {
        for connector in self.connectors.values() {
            connector.start().await?;
        }
        Ok(())
    }

    async fn stop_all(&self) -> Result<()> {
        for connector in self.connectors.values() {
            connector.stop().await?;
        }
        Ok(())
    }

    async fn health_summary(&self) -> Result<Vec<ChannelHealth>> {
        let mut items = Vec::with_capacity(self.connectors.len());
        for connector in self.connectors.values() {
            items.push(connector.health().await?);
        }
        items.sort_by(|a, b| a.channel.cmp(&b.channel));
        Ok(items)
    }

    fn enabled_channels(&self) -> Vec<String> {
        let mut channels = self.connectors.keys().cloned().collect::<Vec<_>>();
        channels.sort();
        channels
    }
}

pub async fn build_dispatcher(config: &AppConfig) -> Result<Option<Arc<dyn ChannelDispatcherPort>>> {
    let mut registry = ChannelDispatcherRegistry::new();

    if config.telegram_enabled {
        let bot_token = config.telegram_bot_token.clone().ok_or_else(|| {
            anyhow!("telegram is enabled but no bot token found; set TELEGRAM_BOT_TOKEN or secrets.telegram_bot_token")
        })?;
        registry.register(Arc::new(TelegramConnector::new(
            bot_token,
            config.telegram_api_base_url.clone(),
        )));
    }

    let enabled = registry.enabled_channels();
    if enabled.is_empty() {
        return Ok(None);
    }

    let dispatcher = Arc::new(registry);
    dispatcher.start_all().await?;
    tracing::info!(channels = ?enabled, "channel dispatcher initialized");
    Ok(Some(dispatcher))
}
