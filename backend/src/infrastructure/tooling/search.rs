use super::{Tool, ToolContext};
use crate::domain::types::ToolExecution;
use crate::infrastructure::config::AppConfig;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_MAX_RESULTS: usize = 5;
const PERPLEXITY_URL: &str = "https://api.perplexity.ai/search";
const TAVILY_URL: &str = "https://api.tavily.com/search";
const BRAVE_URL: &str = "https://api.search.brave.com/res/v1/web/search";

#[derive(Clone)]
pub struct WebSearchTool {
    client: Client,
    config: SearchToolConfig,
    endpoints: SearchEndpoints,
}

#[derive(Clone)]
struct SearchToolConfig {
    provider: Option<String>,
    perplexity_api_key: Option<String>,
    tavily_api_key: Option<String>,
    brave_search_api_key: Option<String>,
}

#[derive(Clone)]
struct SearchEndpoints {
    perplexity_url: String,
    tavily_url: String,
    brave_url: String,
}

#[derive(Debug, Deserialize)]
struct WebSearchArgs {
    #[serde(rename = "queryString")]
    query_string: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct WebSearchResponse {
    provider: String,
    query: String,
    results: Vec<WebSearchResult>,
}

#[derive(Clone, Debug)]
struct SearchRunOutput {
    response: WebSearchResponse,
    is_error: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct WebSearchResult {
    title: String,
    url: String,
    snippet: String,
}

#[derive(Debug, Deserialize)]
struct PerplexityResponse {
    results: Vec<PerplexityResult>,
}

#[derive(Debug, Deserialize)]
struct PerplexityResult {
    title: Option<String>,
    url: Option<String>,
    snippet: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyResult {
    title: Option<String>,
    url: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BraveResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title: Option<String>,
    url: Option<String>,
    description: Option<String>,
}

impl SearchToolConfig {
    fn from_app_config(config: &AppConfig) -> Self {
        Self {
            provider: config.search_provider.clone(),
            perplexity_api_key: config.perplexity_api_key.clone(),
            tavily_api_key: config.tavily_api_key.clone(),
            brave_search_api_key: config.brave_search_api_key.clone(),
        }
    }
}

impl Default for SearchEndpoints {
    fn default() -> Self {
        Self {
            perplexity_url: PERPLEXITY_URL.to_string(),
            tavily_url: TAVILY_URL.to_string(),
            brave_url: BRAVE_URL.to_string(),
        }
    }
}

impl WebSearchTool {
    pub fn new(config: &AppConfig) -> Self {
        Self::with_parts(
            Client::new(),
            SearchToolConfig::from_app_config(config),
            SearchEndpoints::default(),
        )
    }

    fn with_parts(client: Client, config: SearchToolConfig, endpoints: SearchEndpoints) -> Self {
        Self {
            client,
            config,
            endpoints,
        }
    }

    fn provider(&self) -> Result<&str> {
        self.config
            .provider
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("no provider configured"))
    }

    fn provider_api_key(&self, provider: &str) -> Result<&str> {
        match provider {
            "perplexity" => self
                .config
                .perplexity_api_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("missing api key for provider: perplexity")),
            "tavily" => self
                .config
                .tavily_api_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("missing api key for provider: tavily")),
            "brave" => self
                .config
                .brave_search_api_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("missing api key for provider: brave")),
            other => Err(anyhow!("unsupported search provider: {other}")),
        }
    }

    async fn run_search(&self, provider: &str, query: &str) -> Result<SearchRunOutput> {
        match provider {
            "perplexity" => self.search_perplexity(query).await,
            "tavily" => self.search_tavily(query).await,
            "brave" => self.search_brave(query).await,
            other => Err(anyhow!("unsupported search provider: {other}")),
        }
    }

    async fn search_perplexity(&self, query: &str) -> Result<SearchRunOutput> {
        let api_key = self.provider_api_key("perplexity")?;
        let response = match self
            .client
            .post(&self.endpoints.perplexity_url)
            .bearer_auth(api_key)
            .json(&json!({
                "query": query,
                "max_results": DEFAULT_MAX_RESULTS,
            }))
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return Ok(SearchRunOutput {
                    response: error_response(
                        "perplexity",
                        query,
                        format!("web_search provider request failed (perplexity): {error}"),
                    ),
                    is_error: true,
                });
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Ok(SearchRunOutput {
                response: error_response(
                    "perplexity",
                    query,
                    format!(
                        "web_search provider error (perplexity {status}): {}",
                        truncate_error_body(&body)
                    ),
                ),
                is_error: true,
            });
        }

        let payload = match response.json::<PerplexityResponse>().await {
            Ok(payload) => payload,
            Err(error) => {
                return Ok(SearchRunOutput {
                    response: error_response(
                        "perplexity",
                        query,
                        format!("invalid perplexity response body: {error}"),
                    ),
                    is_error: true,
                });
            }
        };

        Ok(SearchRunOutput {
            response: WebSearchResponse {
                provider: "perplexity".to_string(),
                query: query.to_string(),
                results: payload
                    .results
                    .into_iter()
                    .take(DEFAULT_MAX_RESULTS)
                    .map(|item| WebSearchResult {
                        title: item.title.unwrap_or_default(),
                        url: item.url.unwrap_or_default(),
                        snippet: item.snippet.unwrap_or_default(),
                    })
                    .collect(),
            },
            is_error: false,
        })
    }

    async fn search_tavily(&self, query: &str) -> Result<SearchRunOutput> {
        let api_key = self.provider_api_key("tavily")?;
        let response = match self
            .client
            .post(&self.endpoints.tavily_url)
            .bearer_auth(api_key)
            .json(&json!({
                "query": query,
                "topic": "general",
                "search_depth": "basic",
                "max_results": DEFAULT_MAX_RESULTS,
            }))
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return Ok(SearchRunOutput {
                    response: error_response(
                        "tavily",
                        query,
                        format!("web_search provider request failed (tavily): {error}"),
                    ),
                    is_error: true,
                });
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Ok(SearchRunOutput {
                response: error_response(
                    "tavily",
                    query,
                    format!(
                        "web_search provider error (tavily {status}): {}",
                        truncate_error_body(&body)
                    ),
                ),
                is_error: true,
            });
        }

        let payload = match response.json::<TavilyResponse>().await {
            Ok(payload) => payload,
            Err(error) => {
                return Ok(SearchRunOutput {
                    response: error_response(
                        "tavily",
                        query,
                        format!("invalid tavily response body: {error}"),
                    ),
                    is_error: true,
                });
            }
        };

        Ok(SearchRunOutput {
            response: WebSearchResponse {
                provider: "tavily".to_string(),
                query: query.to_string(),
                results: payload
                    .results
                    .into_iter()
                    .take(DEFAULT_MAX_RESULTS)
                    .map(|item| WebSearchResult {
                        title: item.title.unwrap_or_default(),
                        url: item.url.unwrap_or_default(),
                        snippet: item.content.unwrap_or_default(),
                    })
                    .collect(),
            },
            is_error: false,
        })
    }

    async fn search_brave(&self, query: &str) -> Result<SearchRunOutput> {
        let api_key = self.provider_api_key("brave")?;
        let response = match self
            .client
            .get(&self.endpoints.brave_url)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", api_key)
            .query(&[("q", query), ("count", "5")])
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return Ok(SearchRunOutput {
                    response: error_response(
                        "brave",
                        query,
                        format!("web_search provider request failed (brave): {error}"),
                    ),
                    is_error: true,
                });
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Ok(SearchRunOutput {
                response: error_response(
                    "brave",
                    query,
                    format!(
                        "web_search provider error (brave {status}): {}",
                        truncate_error_body(&body)
                    ),
                ),
                is_error: true,
            });
        }

        let payload = match response.json::<BraveResponse>().await {
            Ok(payload) => payload,
            Err(error) => {
                return Ok(SearchRunOutput {
                    response: error_response(
                        "brave",
                        query,
                        format!("invalid brave response body: {error}"),
                    ),
                    is_error: true,
                });
            }
        };

        Ok(SearchRunOutput {
            response: WebSearchResponse {
                provider: "brave".to_string(),
                query: query.to_string(),
                results: payload
                    .web
                    .map(|web| web.results)
                    .unwrap_or_default()
                    .into_iter()
                    .take(DEFAULT_MAX_RESULTS)
                    .map(|item| WebSearchResult {
                        title: item.title.unwrap_or_default(),
                        url: item.url.unwrap_or_default(),
                        snippet: item.description.unwrap_or_default(),
                    })
                    .collect(),
            },
            is_error: false,
        })
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web with the configured search provider"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "queryString": {"type": "string"}
            },
            "required": ["queryString"]
        })
    }

    async fn execute(&self, args: Value, _context: &ToolContext) -> Result<ToolExecution> {
        let parsed: WebSearchArgs =
            serde_json::from_value(args).context("web_search.queryString is required")?;
        let query = parsed.query_string.trim();
        if query.is_empty() {
            return Err(anyhow!("web_search.queryString is required"));
        }

        let provider = self.provider()?;
        let response = self.run_search(provider, query).await?;
        Ok(ToolExecution {
            name: self.name().to_string(),
            output: serde_json::to_string_pretty(&response.response)?,
            is_error: response.is_error,
        })
    }
}

fn error_response(provider: &str, query: &str, message: String) -> WebSearchResponse {
    WebSearchResponse {
        provider: provider.to_string(),
        query: query.to_string(),
        results: vec![WebSearchResult {
            title: "error".to_string(),
            url: String::new(),
            snippet: message,
        }],
    }
}

fn truncate_error_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        "empty response body".to_string()
    } else {
        trimmed.chars().take(240).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::Query,
        http::StatusCode,
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };
    use serde::Deserialize;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::net::TcpListener;

    fn test_context() -> ToolContext {
        let temp = tempdir().unwrap();
        let root = temp.keep();
        let memory = Arc::new(crate::infrastructure::memory::MemoryStore::new(
            root.join("memory"),
            root.join("MEMORY.md"),
        ));
        ToolContext::new(root, memory)
    }

    #[derive(Debug, Deserialize)]
    struct BraveQuery {
        q: String,
        count: Option<String>,
    }

    async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
        async fn perplexity_handler(Json(payload): Json<Value>) -> impl IntoResponse {
            let query = payload
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default();
            Json(json!({
                "results": [{
                    "title": format!("Perplexity: {query}"),
                    "url": "https://perplexity.example/result",
                    "snippet": "perplexity snippet"
                }]
            }))
        }

        async fn tavily_handler(Json(payload): Json<Value>) -> impl IntoResponse {
            let query = payload
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default();
            Json(json!({
                "results": [{
                    "title": format!("Tavily: {query}"),
                    "url": "https://tavily.example/result",
                    "content": "tavily snippet"
                }]
            }))
        }

        async fn brave_handler(Query(query): Query<BraveQuery>) -> impl IntoResponse {
            Json(json!({
                "web": {
                    "results": [{
                        "title": format!("Brave: {}", query.q),
                        "url": "https://brave.example/result",
                        "description": format!("count={}", query.count.unwrap_or_default())
                    }]
                }
            }))
        }

        async fn failing_handler() -> impl IntoResponse {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "upstream failed" })),
            )
        }

        let app = Router::new()
            .route("/perplexity", post(perplexity_handler))
            .route("/tavily", post(tavily_handler))
            .route("/brave", get(brave_handler))
            .route("/fail", get(failing_handler).post(failing_handler));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{}", addr), handle)
    }

    fn tool_with_provider(provider: &str, base_url: &str) -> WebSearchTool {
        let config = SearchToolConfig {
            provider: Some(provider.to_string()),
            perplexity_api_key: Some("pplx-key".to_string()),
            tavily_api_key: Some("tvly-key".to_string()),
            brave_search_api_key: Some("brave-key".to_string()),
        };
        let endpoints = SearchEndpoints {
            perplexity_url: format!("{base_url}/perplexity"),
            tavily_url: format!("{base_url}/tavily"),
            brave_url: format!("{base_url}/brave"),
        };
        WebSearchTool::with_parts(Client::new(), config, endpoints)
    }

    #[tokio::test]
    async fn execute_requires_configured_provider() {
        let tool = WebSearchTool::with_parts(
            Client::new(),
            SearchToolConfig {
                provider: None,
                perplexity_api_key: None,
                tavily_api_key: None,
                brave_search_api_key: None,
            },
            SearchEndpoints::default(),
        );
        let context = test_context();
        let error = tool
            .execute(json!({"queryString": "rust"}), &context)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "no provider configured");
    }

    #[tokio::test]
    async fn execute_errors_when_provider_key_missing() {
        let tool = WebSearchTool::with_parts(
            Client::new(),
            SearchToolConfig {
                provider: Some("tavily".to_string()),
                perplexity_api_key: None,
                tavily_api_key: None,
                brave_search_api_key: None,
            },
            SearchEndpoints::default(),
        );
        let context = test_context();
        let error = tool
            .execute(json!({"queryString": "rust"}), &context)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "missing api key for provider: tavily");
    }

    #[tokio::test]
    async fn execute_normalizes_perplexity_results() {
        let (base_url, _server) = spawn_test_server().await;
        let tool = tool_with_provider("perplexity", &base_url);
        let context = test_context();
        let execution = tool
            .execute(json!({"queryString": "rust async"}), &context)
            .await
            .unwrap();
        assert!(!execution.is_error);
        let payload: Value = serde_json::from_str(&execution.output).unwrap();
        assert_eq!(payload["provider"], "perplexity");
        assert_eq!(payload["query"], "rust async");
        assert_eq!(payload["results"][0]["title"], "Perplexity: rust async");
        assert_eq!(payload["results"][0]["snippet"], "perplexity snippet");
    }

    #[tokio::test]
    async fn execute_normalizes_tavily_results() {
        let (base_url, _server) = spawn_test_server().await;
        let tool = tool_with_provider("tavily", &base_url);
        let context = test_context();
        let execution = tool
            .execute(json!({"queryString": "rust async"}), &context)
            .await
            .unwrap();
        assert!(!execution.is_error);
        let payload: Value = serde_json::from_str(&execution.output).unwrap();
        assert_eq!(payload["provider"], "tavily");
        assert_eq!(payload["results"][0]["title"], "Tavily: rust async");
        assert_eq!(payload["results"][0]["snippet"], "tavily snippet");
    }

    #[tokio::test]
    async fn execute_normalizes_brave_results() {
        let (base_url, _server) = spawn_test_server().await;
        let tool = tool_with_provider("brave", &base_url);
        let context = test_context();
        let execution = tool
            .execute(json!({"queryString": "rust async"}), &context)
            .await
            .unwrap();
        assert!(!execution.is_error);
        let payload: Value = serde_json::from_str(&execution.output).unwrap();
        assert_eq!(payload["provider"], "brave");
        assert_eq!(payload["results"][0]["title"], "Brave: rust async");
        assert_eq!(payload["results"][0]["snippet"], "count=5");
    }

    #[tokio::test]
    async fn execute_returns_error_payload_for_upstream_failure() {
        let (base_url, _server) = spawn_test_server().await;
        let config = SearchToolConfig {
            provider: Some("brave".to_string()),
            perplexity_api_key: Some("pplx-key".to_string()),
            tavily_api_key: Some("tvly-key".to_string()),
            brave_search_api_key: Some("brave-key".to_string()),
        };
        let endpoints = SearchEndpoints {
            perplexity_url: format!("{base_url}/fail"),
            tavily_url: format!("{base_url}/fail"),
            brave_url: format!("{base_url}/fail"),
        };
        let tool = WebSearchTool::with_parts(Client::new(), config, endpoints);
        let context = test_context();
        let execution = tool
            .execute(json!({"queryString": "rust async"}), &context)
            .await
            .unwrap();
        assert!(execution.is_error);
        let payload: Value = serde_json::from_str(&execution.output).unwrap();
        assert_eq!(payload["provider"], "brave");
        assert_eq!(payload["results"][0]["title"], "error");
        assert!(payload["results"][0]["snippet"]
            .as_str()
            .unwrap()
            .contains("web_search provider error"));
    }
}
