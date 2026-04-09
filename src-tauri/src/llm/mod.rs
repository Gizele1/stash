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
    Cloud,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub mode: LlmMode,
    pub local_model: String,
    pub cloud_model: String,
    pub direction_change_threshold: f64,
    pub max_retries: u32,
    pub initial_backoff_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            mode: LlmMode::Auto,
            local_model: "llama3".to_string(),
            cloud_model: "claude-sonnet".to_string(),
            direction_change_threshold: 0.7,
            max_retries: 3,
            initial_backoff_secs: 1,
        }
    }
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
