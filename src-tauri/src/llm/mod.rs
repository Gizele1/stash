use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntentTier {
    Narrative,
    Summary,
    Label,
}

impl IntentTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntentTier::Narrative => "narrative",
            IntentTier::Summary => "summary",
            IntentTier::Label => "label",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "narrative" => Some(IntentTier::Narrative),
            "summary" => Some(IntentTier::Summary),
            "label" => Some(IntentTier::Label),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Workload {
    Distillation,
    Compression,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LlmMode {
    Local,
    Hybrid,
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub mode: LlmMode,
    pub local_model: String,
    pub cloud_model: String,
    pub direction_change_threshold: f64,
    pub max_retries: u32,
    pub initial_backoff_secs: u64,
    pub ollama_url: String,
    pub cloud_provider: String,
    pub cloud_api_key: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            mode: LlmMode::Hybrid,
            local_model: "llama3".to_string(),
            cloud_model: "claude-sonnet".to_string(),
            direction_change_threshold: 0.7,
            max_retries: 3,
            initial_backoff_secs: 1,
            ollama_url: "http://localhost:11434".to_string(),
            cloud_provider: "anthropic".to_string(),
            cloud_api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillResult {
    pub narrative: String,
    pub is_direction_change: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualHealthStatus {
    pub ollama_available: bool,
    pub cloud_available: bool,
    pub active_mode: String,
}

#[derive(Debug, Clone)]
pub struct ProviderRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone)]
pub struct ProviderReply {
    pub content: String,
    pub finish_reason: String,
}

#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub available: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Provider unavailable: {0}")]
    Unavailable(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Timeout")]
    Timeout,
}

// ── Trait ──

pub trait LlmProvider: Send + Sync {
    fn health_check(&self) -> ProviderHealth;
    fn generate(&self, request: ProviderRequest) -> Result<ProviderReply, ProviderError>;
}

// ── Router ──

pub struct LlmRouter {
    provider: Arc<dyn LlmProvider>,
    config: LlmConfig,
}

impl LlmRouter {
    pub fn new(provider: Arc<dyn LlmProvider>, config: LlmConfig) -> Self {
        Self { provider, config }
    }

    pub fn route(&self, _workload: Workload, request: ProviderRequest) -> Result<ProviderReply, ProviderError> {
        // For now, route all to the single provider
        // Future: choose between local/cloud based on workload + config
        let health = self.provider.health_check();
        if !health.available {
            return Err(ProviderError::Unavailable("No provider available".to_string()));
        }
        self.provider.generate(request)
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    pub fn provider_health(&self) -> ProviderHealth {
        self.provider.health_check()
    }

    pub fn distill(
        &self,
        prompts: Vec<String>,
        previous_intent: Option<String>,
        _language_hint: Option<String>,
    ) -> Result<DistillResult, ProviderError> {
        let context = if let Some(prev) = &previous_intent {
            format!("Previous intent: {}\n\n", prev)
        } else {
            String::new()
        };

        let user_prompt = format!(
            "{}Prompts to distill:\n{}",
            context,
            prompts.join("\n")
        );

        let request = ProviderRequest {
            system_prompt: "You are a distillation engine. Summarize the prompts into a concise narrative. Respond with JSON: {\"narrative\": \"...\", \"direction_change\": bool, \"confidence\": float}".to_string(),
            user_prompt,
            max_tokens: 512,
            temperature: 0.3,
        };

        let reply = self.route(Workload::Distillation, request)?;

        // Parse JSON response; fall back to using raw content as narrative
        let result = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&reply.content) {
            DistillResult {
                narrative: v["narrative"].as_str().unwrap_or(&reply.content).to_string(),
                is_direction_change: v["direction_change"].as_bool().unwrap_or(false),
                confidence: v["confidence"].as_f64().unwrap_or(0.8) as f32,
            }
        } else {
            DistillResult {
                narrative: reply.content,
                is_direction_change: false,
                confidence: 0.8,
            }
        };

        Ok(result)
    }

    pub fn compress_batch(
        &self,
        intents: Vec<String>,
        target_tier: &str,
    ) -> Result<String, ProviderError> {
        let user_prompt = format!(
            "Compress the following intents into a single {} summary:\n{}",
            target_tier,
            intents.join("\n")
        );

        let request = ProviderRequest {
            system_prompt: "You are a compression engine. Compress the given intents into a concise summary at the requested tier level.".to_string(),
            user_prompt,
            max_tokens: 256,
            temperature: 0.2,
        };

        let reply = self.route(Workload::Compression, request)?;
        Ok(reply.content)
    }

    pub fn dual_health_check(&self) -> DualHealthStatus {
        let health = self.provider.health_check();
        let active_mode = match self.config.mode {
            LlmMode::Local => "local",
            LlmMode::Hybrid => "hybrid",
            LlmMode::Cloud => "cloud",
        };
        DualHealthStatus {
            ollama_available: health.available,
            cloud_available: false,
            active_mode: active_mode.to_string(),
        }
    }
}

// ── Retry Queue (bounded) ──

pub struct RetryQueue {
    capacity: usize,
    items: std::sync::Mutex<Vec<RetryItem>>,
}

pub struct RetryItem {
    pub request: ProviderRequest,
    pub workload: Workload,
    pub attempt: u32,
    pub next_retry_at: chrono::DateTime<chrono::Utc>,
}

impl RetryQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            items: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn push(&self, item: RetryItem) -> Result<(), &'static str> {
        let mut items = self.items.lock().expect("mutex poisoned");
        if items.len() >= self.capacity {
            return Err("Queue full");
        }
        items.push(item);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.items.lock().expect("mutex poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ── Stub Provider (used at runtime until a real LLM backend is configured) ──

/// A stub LLM provider that returns deterministic responses.
/// Suitable for development and testing without a real LLM backend.
pub struct StubLlmProvider;

impl StubLlmProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LlmProvider for StubLlmProvider {
    fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            available: true,
            latency_ms: Some(0),
        }
    }

    fn generate(&self, request: ProviderRequest) -> Result<ProviderReply, ProviderError> {
        // Return a deterministic distillation/compression response based on prompt content
        let content = if request.system_prompt.contains("distill") || request.system_prompt.contains("summarize") {
            serde_json::json!({
                "narrative": format!("(stub) {}", &request.user_prompt[..request.user_prompt.len().min(120)]),
                "direction_change": false
            }).to_string()
        } else {
            format!("(stub) compressed: {}", &request.user_prompt[..request.user_prompt.len().min(120)])
        };

        Ok(ProviderReply {
            content,
            finish_reason: "stop".to_string(),
        })
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Mutex;

    /// A mock LLM provider for testing
    pub struct MockLlmProvider {
        responses: Mutex<Vec<Result<ProviderReply, ProviderError>>>,
        available: Mutex<bool>,
    }

    impl MockLlmProvider {
        pub fn new() -> Self {
            Self {
                responses: Mutex::new(Vec::new()),
                available: Mutex::new(true),
            }
        }

        pub fn set_available(&self, available: bool) {
            *self.available.lock().unwrap() = available;
        }

        pub fn enqueue_response(&self, reply: Result<ProviderReply, ProviderError>) {
            self.responses.lock().unwrap().push(reply);
        }

        pub fn enqueue_distill_response(&self, content: &str, direction_change: bool) {
            let json = serde_json::json!({
                "narrative": content,
                "direction_change": direction_change
            });
            self.enqueue_response(Ok(ProviderReply {
                content: json.to_string(),
                finish_reason: "stop".to_string(),
            }));
        }

        pub fn enqueue_compress_response(&self, content: &str) {
            self.enqueue_response(Ok(ProviderReply {
                content: content.to_string(),
                finish_reason: "stop".to_string(),
            }));
        }
    }

    impl LlmProvider for MockLlmProvider {
        fn health_check(&self) -> ProviderHealth {
            let available = *self.available.lock().unwrap();
            ProviderHealth {
                available,
                latency_ms: if available { Some(10) } else { None },
            }
        }

        fn generate(&self, _request: ProviderRequest) -> Result<ProviderReply, ProviderError> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(ProviderError::RequestFailed("No mock responses queued".to_string()));
            }
            responses.remove(0)
        }
    }
}
