use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};

const NARRATIVE_CHAR_LIMIT: usize = 200;

pub trait LlmProvider: Send + Sync {
    fn health_check(&self) -> ProviderHealth;
    fn generate(&self, request: ProviderRequest) -> Result<ProviderReply, ProviderError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmMode {
    Local,
    Hybrid,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Local,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workload {
    Distillation,
    Compression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentTier {
    Narrative,
    Summary,
    Label,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmConfig {
    pub mode: LlmMode,
    pub local_model: String,
    pub cloud_model: Option<String>,
    pub direction_change_threshold: f32,
    pub max_retries: u8,
    pub initial_backoff_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderHealth {
    pub available: bool,
    pub message: String,
    pub setup_guide: Option<String>,
}

impl ProviderHealth {
    pub fn available(message: &str) -> Self {
        Self {
            available: true,
            message: message.to_string(),
            setup_guide: None,
        }
    }

    pub fn unavailable(message: &str, setup_guide: Option<&str>) -> Self {
        Self {
            available: false,
            message: message.to_string(),
            setup_guide: setup_guide.map(ToString::to_string),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    Unavailable {
        message: String,
        setup_guide: Option<String>,
    },
    Timeout,
    ResponseParseError(String),
    Failed(String),
}

impl ProviderError {
    pub fn unavailable(message: &str, setup_guide: Option<&str>) -> Self {
        Self::Unavailable {
            message: message.to_string(),
            setup_guide: setup_guide.map(ToString::to_string),
        }
    }
}

impl Display for ProviderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { message, .. } => write!(f, "provider unavailable: {message}"),
            Self::Timeout => write!(f, "provider timed out"),
            Self::ResponseParseError(message) => {
                write!(f, "provider response parse error: {message}")
            }
            Self::Failed(message) => write!(f, "provider failed: {message}"),
        }
    }
}

impl Error for ProviderError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    MissingProvider(ProviderKind),
    InvalidRequest(String),
    Timeout {
        provider: ProviderKind,
    },
    ResponseParseError {
        provider: ProviderKind,
        message: String,
    },
    ProviderFailure {
        provider: ProviderKind,
        source: ProviderError,
    },
}

impl Display for LlmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProvider(kind) => write!(f, "missing {:?} provider", kind),
            Self::InvalidRequest(message) => write!(f, "invalid request: {message}"),
            Self::Timeout { provider } => write!(f, "{provider:?} provider timed out"),
            Self::ResponseParseError { provider, message } => {
                write!(f, "{provider:?} provider response parse error: {message}")
            }
            Self::ProviderFailure { provider, source } => {
                write!(f, "{provider:?} provider error: {source}")
            }
        }
    }
}

impl Error for LlmError {}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderRequest {
    pub workload: Workload,
    pub context_id: String,
    pub raw_prompts: Vec<String>,
    pub input_intents: Vec<IntentRecord>,
    pub previous_intent: Option<String>,
    pub language_hint: Option<String>,
    pub model: String,
    pub direction_change_threshold: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderReply {
    pub content: String,
    pub similarity_score: Option<f32>,
}

impl ProviderReply {
    pub fn distillation(content: &str, similarity_score: Option<f32>) -> Self {
        Self {
            content: content.to_string(),
            similarity_score,
        }
    }

    pub fn compression(content: &str) -> Self {
        Self {
            content: content.to_string(),
            similarity_score: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentRecord {
    pub id: String,
    pub content: String,
    pub tier: IntentTier,
    pub created_at_secs: u64,
    pub archived: bool,
    pub compressed_from: Vec<String>,
}

impl IntentRecord {
    pub fn narrative(id: &str, content: &str, created_at_secs: u64) -> Self {
        Self::new(id, content, IntentTier::Narrative, created_at_secs)
    }

    pub fn summary(id: &str, content: &str, created_at_secs: u64) -> Self {
        Self::new(id, content, IntentTier::Summary, created_at_secs)
    }

    fn new(id: &str, content: &str, tier: IntentTier, created_at_secs: u64) -> Self {
        Self {
            id: id.to_string(),
            content: content.to_string(),
            tier,
            created_at_secs,
            archived: false,
            compressed_from: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistillationRequest {
    pub context_id: String,
    pub raw_prompts: Vec<String>,
    pub previous_intent: Option<String>,
    pub language_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistillationDisposition {
    Completed,
    Queued,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistilledIntent {
    pub content: String,
    pub confidence: f32,
    pub is_direction_change: bool,
    pub marker: Option<String>,
}

impl DistilledIntent {
    fn empty() -> Self {
        Self {
            content: String::new(),
            confidence: 0.0,
            is_direction_change: false,
            marker: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistillationOutcome {
    pub disposition: DistillationDisposition,
    pub intent: DistilledIntent,
    pub confidence: f32,
    pub provider: ProviderKind,
    pub retry_after_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionRequest {
    pub context_id: String,
    pub intents: Vec<IntentRecord>,
    pub target_tier: IntentTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionOutcome {
    pub provider: ProviderKind,
    pub compressed_intent: IntentRecord,
    pub archived_sources: Vec<IntentRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    pub local: ProviderHealth,
    pub cloud: ProviderHealth,
    pub degraded_to_cloud_only: bool,
    pub setup_guide: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryOutcome {
    pub context_id: String,
    pub attempt: u8,
    pub next_retry_at_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedDistillation {
    pub request: DistillationRequest,
    pub attempt: u8,
    pub next_retry_at_secs: u64,
    pub last_error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmStatus {
    pub local: ProviderHealth,
    pub cloud: ProviderHealth,
    pub degraded_to_cloud_only: bool,
    pub setup_guide: Option<String>,
    pub loading_contexts: Vec<String>,
    pub queue: Vec<QueuedDistillation>,
}

#[derive(Default)]
struct EngineState {
    loading_contexts: BTreeSet<String>,
    queue: VecDeque<QueuedDistillation>,
    degraded_to_cloud_only: bool,
    setup_guide: Option<String>,
}

pub struct LlmEngine {
    config: LlmConfig,
    local: Option<Arc<dyn LlmProvider>>,
    cloud: Option<Arc<dyn LlmProvider>>,
    state: Mutex<EngineState>,
}

impl LlmEngine {
    pub fn new(
        config: LlmConfig,
        local: Option<Box<dyn LlmProvider>>,
        cloud: Option<Box<dyn LlmProvider>>,
    ) -> Self {
        Self {
            config,
            local: local.map(Arc::from),
            cloud: cloud.map(Arc::from),
            state: Mutex::new(EngineState::default()),
        }
    }

    pub fn distill(
        &self,
        request: DistillationRequest,
        now_secs: u64,
    ) -> Result<DistillationOutcome, LlmError> {
        let route = self.route_for(Workload::Distillation)?;
        self.set_loading(&request.context_id, true);

        let primary_result = self.call_provider(
            route.primary,
            ProviderRequest {
                workload: Workload::Distillation,
                context_id: request.context_id.clone(),
                raw_prompts: request.raw_prompts.clone(),
                input_intents: Vec::new(),
                previous_intent: request.previous_intent.clone(),
                language_hint: request.language_hint.clone(),
                model: route.model.clone(),
                direction_change_threshold: Some(self.config.direction_change_threshold),
            },
        );

        let outcome = match primary_result {
            Ok(reply) => {
                self.update_degradation(false, None);
                Ok(self.completed_distillation(route.primary, reply))
            }
            Err(ProviderError::Unavailable {
                message,
                setup_guide,
            }) => {
                if let Some(fallback) = route.fallback {
                    let fallback_result = self.call_provider(
                        fallback,
                        ProviderRequest {
                            workload: Workload::Distillation,
                            context_id: request.context_id.clone(),
                            raw_prompts: request.raw_prompts.clone(),
                            input_intents: Vec::new(),
                            previous_intent: request.previous_intent.clone(),
                            language_hint: request.language_hint.clone(),
                            model: self.model_for(fallback)?,
                            direction_change_threshold: Some(
                                self.config.direction_change_threshold,
                            ),
                        },
                    );

                    match fallback_result {
                        Ok(reply) => {
                            self.update_degradation(true, setup_guide);
                            Ok(self.completed_distillation(fallback, reply))
                        }
                        Err(source) => Err(Self::map_provider_error(fallback, source)),
                    }
                } else {
                    let retry_after_secs = now_secs + self.backoff_for_attempt(1);
                    self.enqueue_distillation(QueuedDistillation {
                        request: request.clone(),
                        attempt: 1,
                        next_retry_at_secs: retry_after_secs,
                        last_error: message,
                    });
                    self.update_degradation(false, setup_guide);
                    Ok(DistillationOutcome {
                        disposition: DistillationDisposition::Queued,
                        intent: DistilledIntent::empty(),
                        confidence: 0.0,
                        provider: route.primary,
                        retry_after_secs: Some(retry_after_secs),
                    })
                }
            }
            Err(source) => Err(Self::map_provider_error(route.primary, source)),
        };

        self.set_loading(&request.context_id, false);
        outcome
    }

    pub fn retry_queued_distillations(&self, now_secs: u64) -> Result<Vec<RetryOutcome>, LlmError> {
        let due = {
            let mut state = self.state.lock().unwrap();
            let mut remaining = VecDeque::new();
            let mut due = Vec::new();

            while let Some(item) = state.queue.pop_front() {
                if item.next_retry_at_secs <= now_secs {
                    due.push(item);
                } else {
                    remaining.push_back(item);
                }
            }

            state.queue = remaining;
            due
        };

        let route = self.route_for(Workload::Distillation)?;
        let mut results = Vec::new();

        for item in due {
            let context_id = item.request.context_id.clone();
            self.set_loading(&context_id, true);

            let result = self.call_provider(
                route.primary,
                ProviderRequest {
                    workload: Workload::Distillation,
                    context_id: context_id.clone(),
                    raw_prompts: item.request.raw_prompts.clone(),
                    input_intents: Vec::new(),
                    previous_intent: item.request.previous_intent.clone(),
                    language_hint: item.request.language_hint.clone(),
                    model: route.model.clone(),
                    direction_change_threshold: Some(self.config.direction_change_threshold),
                },
            );

            match result {
                Ok(_) => {
                    results.push(RetryOutcome {
                        context_id: context_id.clone(),
                        attempt: item.attempt + 1,
                        next_retry_at_secs: None,
                    });
                }
                Err(ProviderError::Unavailable {
                    message,
                    setup_guide,
                }) => {
                    let next_retry_at_secs = if item.attempt < self.config.max_retries {
                        let next = now_secs + self.backoff_for_attempt(item.attempt + 1);
                        self.enqueue_distillation(QueuedDistillation {
                            request: item.request,
                            attempt: item.attempt + 1,
                            next_retry_at_secs: next,
                            last_error: message,
                        });
                        Some(next)
                    } else {
                        None
                    };
                    self.update_degradation(false, setup_guide);
                    results.push(RetryOutcome {
                        context_id: context_id.clone(),
                        attempt: item.attempt + 1,
                        next_retry_at_secs,
                    });
                }
                Err(source) => {
                    self.set_loading(&context_id, false);
                    return Err(Self::map_provider_error(route.primary, source));
                }
            }

            self.set_loading(&context_id, false);
        }

        Ok(results)
    }

    pub fn compress_batch(
        &self,
        request: CompressionRequest,
        now_secs: u64,
    ) -> Result<Option<CompressionOutcome>, LlmError> {
        if request.intents.is_empty() {
            return Ok(None);
        }

        self.validate_compression_request(&request)?;

        let route = self.route_for(Workload::Compression)?;
        let reply = self
            .call_provider(
                route.primary,
                ProviderRequest {
                    workload: Workload::Compression,
                    context_id: request.context_id.clone(),
                    raw_prompts: Vec::new(),
                    input_intents: request.intents.clone(),
                    previous_intent: None,
                    language_hint: None,
                    model: route.model,
                    direction_change_threshold: None,
                },
            )
            .map_err(|source| Self::map_provider_error(route.primary, source))?;

        let archived_sources: Vec<IntentRecord> = request
            .intents
            .iter()
            .cloned()
            .map(|mut intent| {
                intent.archived = true;
                intent
            })
            .collect();

        let compressed_intent = IntentRecord {
            id: format!("cmp-{}-{now_secs}", request.context_id),
            content: reply.content.trim().to_string(),
            tier: request.target_tier,
            created_at_secs: now_secs,
            archived: false,
            compressed_from: request
                .intents
                .iter()
                .map(|intent| intent.id.clone())
                .collect(),
        };

        Ok(Some(CompressionOutcome {
            provider: route.primary,
            compressed_intent,
            archived_sources,
        }))
    }

    pub fn health_check(&self) -> Result<HealthReport, LlmError> {
        let local = self.provider_health(ProviderKind::Local);
        let cloud = self.provider_health(ProviderKind::Cloud);
        let degraded_to_cloud_only = self.should_degrade_to_cloud_only(&local, &cloud);
        let setup_guide = local
            .setup_guide
            .clone()
            .or_else(|| cloud.setup_guide.clone());

        self.update_degradation(degraded_to_cloud_only, setup_guide.clone());

        Ok(HealthReport {
            local,
            cloud,
            degraded_to_cloud_only,
            setup_guide,
        })
    }

    pub fn get_llm_status(&self) -> Result<LlmStatus, LlmError> {
        let health = self.health_check()?;
        let state = self.state.lock().unwrap();

        Ok(LlmStatus {
            local: health.local,
            cloud: health.cloud,
            degraded_to_cloud_only: state.degraded_to_cloud_only,
            setup_guide: state.setup_guide.clone(),
            loading_contexts: state.loading_contexts.iter().cloned().collect(),
            queue: state.queue.iter().cloned().collect(),
        })
    }

    fn completed_distillation(
        &self,
        provider: ProviderKind,
        reply: ProviderReply,
    ) -> DistillationOutcome {
        let confidence = reply.similarity_score.unwrap_or(1.0);
        let content = truncate_to_chars(reply.content.trim(), NARRATIVE_CHAR_LIMIT);
        let is_direction_change = confidence < self.config.direction_change_threshold;

        DistillationOutcome {
            disposition: DistillationDisposition::Completed,
            intent: DistilledIntent {
                content,
                confidence,
                is_direction_change,
                marker: is_direction_change.then(|| "转折".to_string()),
            },
            confidence,
            provider,
            retry_after_secs: None,
        }
    }

    fn validate_compression_request(
        &self,
        request: &CompressionRequest,
    ) -> Result<(), LlmError> {
        let source_tier = request.intents[0].tier;
        if request.intents.iter().any(|intent| intent.tier != source_tier) {
            return Err(LlmError::InvalidRequest(
                "compression batch must contain intents from a single tier".to_string(),
            ));
        }

        let expected_target = match source_tier {
            IntentTier::Narrative => Some(IntentTier::Summary),
            IntentTier::Summary => Some(IntentTier::Label),
            IntentTier::Label => None,
        };

        match expected_target {
            Some(expected) if expected == request.target_tier => Ok(()),
            Some(expected) => Err(LlmError::InvalidRequest(format!(
                "cannot compress {:?} intents into {:?}; expected {:?}",
                source_tier, request.target_tier, expected
            ))),
            None => Err(LlmError::InvalidRequest(
                "label intents cannot be compressed further".to_string(),
            )),
        }
    }

    fn map_provider_error(provider: ProviderKind, source: ProviderError) -> LlmError {
        match source {
            ProviderError::Timeout => LlmError::Timeout { provider },
            ProviderError::ResponseParseError(message) => {
                LlmError::ResponseParseError { provider, message }
            }
            other => LlmError::ProviderFailure {
                provider,
                source: other,
            },
        }
    }

    fn route_for(&self, workload: Workload) -> Result<ProviderRoute, LlmError> {
        match (self.config.mode, workload) {
            (LlmMode::Local, _) => Ok(ProviderRoute {
                primary: ProviderKind::Local,
                fallback: None,
                model: self.model_for(ProviderKind::Local)?,
            }),
            (LlmMode::Hybrid, Workload::Distillation) => Ok(ProviderRoute {
                primary: ProviderKind::Local,
                fallback: self.cloud.as_ref().map(|_| ProviderKind::Cloud),
                model: self.model_for(ProviderKind::Local)?,
            }),
            (LlmMode::Hybrid, Workload::Compression) => Ok(ProviderRoute {
                primary: ProviderKind::Cloud,
                fallback: None,
                model: self.model_for(ProviderKind::Cloud)?,
            }),
            (LlmMode::Cloud, _) => Ok(ProviderRoute {
                primary: ProviderKind::Cloud,
                fallback: None,
                model: self.model_for(ProviderKind::Cloud)?,
            }),
        }
    }

    fn model_for(&self, provider: ProviderKind) -> Result<String, LlmError> {
        match provider {
            ProviderKind::Local => {
                if self.local.is_none() {
                    Err(LlmError::MissingProvider(ProviderKind::Local))
                } else {
                    Ok(self.config.local_model.clone())
                }
            }
            ProviderKind::Cloud => {
                if self.cloud.is_none() {
                    Err(LlmError::MissingProvider(ProviderKind::Cloud))
                } else {
                    self.config
                        .cloud_model
                        .clone()
                        .ok_or(LlmError::MissingProvider(ProviderKind::Cloud))
                }
            }
        }
    }

    fn call_provider(
        &self,
        provider: ProviderKind,
        request: ProviderRequest,
    ) -> Result<ProviderReply, ProviderError> {
        match provider {
            ProviderKind::Local => self
                .local
                .as_ref()
                .ok_or_else(|| ProviderError::Failed("local provider not configured".to_string()))?
                .generate(request),
            ProviderKind::Cloud => self
                .cloud
                .as_ref()
                .ok_or_else(|| ProviderError::Failed("cloud provider not configured".to_string()))?
                .generate(request),
        }
    }

    fn provider_health(&self, provider: ProviderKind) -> ProviderHealth {
        match provider {
            ProviderKind::Local => self
                .local
                .as_ref()
                .map(|provider| provider.health_check())
                .unwrap_or_else(|| {
                    ProviderHealth::unavailable("local provider not configured", None)
                }),
            ProviderKind::Cloud => self
                .cloud
                .as_ref()
                .map(|provider| provider.health_check())
                .unwrap_or_else(|| {
                    ProviderHealth::unavailable("cloud provider not configured", None)
                }),
        }
    }

    fn should_degrade_to_cloud_only(&self, local: &ProviderHealth, cloud: &ProviderHealth) -> bool {
        matches!(self.config.mode, LlmMode::Hybrid) && !local.available && cloud.available
    }

    fn update_degradation(&self, degraded_to_cloud_only: bool, setup_guide: Option<String>) {
        let mut state = self.state.lock().unwrap();
        state.degraded_to_cloud_only = degraded_to_cloud_only;
        state.setup_guide = setup_guide;
    }

    fn set_loading(&self, context_id: &str, loading: bool) {
        let mut state = self.state.lock().unwrap();
        if loading {
            state.loading_contexts.insert(context_id.to_string());
        } else {
            state.loading_contexts.remove(context_id);
        }
    }

    fn enqueue_distillation(&self, queued: QueuedDistillation) {
        self.state.lock().unwrap().queue.push_back(queued);
    }

    fn backoff_for_attempt(&self, attempt: u8) -> u64 {
        let exponent = u32::from(attempt.saturating_sub(1));
        self.config
            .initial_backoff_secs
            .saturating_mul(1_u64 << exponent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderRoute {
    primary: ProviderKind,
    fallback: Option<ProviderKind>,
    model: String,
}

fn truncate_to_chars(input: &str, limit: usize) -> String {
    input.chars().take(limit).collect()
}

// ── Default for LlmConfig ──

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            mode: LlmMode::Local,
            local_model: "qwen2.5:7b".to_string(),
            cloud_model: None,
            direction_change_threshold: 0.3,
            max_retries: 3,
            initial_backoff_secs: 5,
        }
    }
}

// ── StubLlmProvider (returns static responses for dev/testing) ──

pub struct StubLlmProvider;

impl Default for StubLlmProvider {
    fn default() -> Self {
        Self
    }
}

impl StubLlmProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LlmProvider for StubLlmProvider {
    fn health_check(&self) -> ProviderHealth {
        ProviderHealth::available("stub provider (no real LLM)")
    }

    fn generate(&self, _request: ProviderRequest) -> Result<ProviderReply, ProviderError> {
        Ok(ProviderReply {
            content: r#"{"narrative": "Development in progress", "direction_change": false}"#
                .to_string(),
            similarity_score: Some(1.0),
        })
    }
}

// ── RouterRequest + LlmRouter (simple adapter for Brain module) ──

/// Simple request type used by Brain's distiller/compressor via LlmRouter.
pub struct RouterRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

/// Lightweight router that wraps a single LlmProvider for Brain's use.
/// Unlike LlmEngine (which has retry queues, fallback routing, etc.),
/// LlmRouter simply calls the provider directly.
pub struct LlmRouter {
    provider: Arc<dyn LlmProvider>,
    config: LlmConfig,
}

impl LlmRouter {
    pub fn new(provider: Arc<dyn LlmProvider>, config: LlmConfig) -> Self {
        Self { provider, config }
    }

    pub fn route(
        &self,
        workload: Workload,
        request: RouterRequest,
    ) -> Result<ProviderReply, ProviderError> {
        let provider_request = ProviderRequest {
            workload,
            context_id: String::new(),
            raw_prompts: vec![format!(
                "{}\n\n{}",
                request.system_prompt, request.user_prompt
            )],
            input_intents: Vec::new(),
            previous_intent: None,
            language_hint: None,
            model: self.config.local_model.clone(),
            direction_change_threshold: Some(self.config.direction_change_threshold),
        };
        self.provider.generate(provider_request)
    }

    pub fn provider_health(&self) -> ProviderHealth {
        self.provider.health_check()
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }
}

// ── Mock module for Brain tests ──

pub mod mock {
    use super::*;
    use std::collections::VecDeque;

    pub struct MockLlmProvider {
        available: Mutex<bool>,
        responses: Mutex<VecDeque<Result<ProviderReply, ProviderError>>>,
    }

    impl Default for MockLlmProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockLlmProvider {
        pub fn new() -> Self {
            Self {
                available: Mutex::new(true),
                responses: Mutex::new(VecDeque::new()),
            }
        }

        pub fn set_available(&self, available: bool) {
            *self.available.lock().unwrap() = available;
        }

        pub fn enqueue_distill_response(&self, narrative: &str, direction_change: bool) {
            let json = serde_json::json!({
                "narrative": narrative,
                "direction_change": direction_change,
            });
            self.responses
                .lock()
                .unwrap()
                .push_back(Ok(ProviderReply {
                    content: json.to_string(),
                    similarity_score: Some(if direction_change { 0.2 } else { 0.9 }),
                }));
        }

        pub fn enqueue_compress_response(&self, content: &str) {
            self.responses
                .lock()
                .unwrap()
                .push_back(Ok(ProviderReply {
                    content: content.to_string(),
                    similarity_score: None,
                }));
        }
    }

    impl LlmProvider for MockLlmProvider {
        fn health_check(&self) -> ProviderHealth {
            if *self.available.lock().unwrap() {
                ProviderHealth::available("mock provider")
            } else {
                ProviderHealth::unavailable("mock unavailable", None)
            }
        }

        fn generate(&self, _request: ProviderRequest) -> Result<ProviderReply, ProviderError> {
            if !*self.available.lock().unwrap() {
                return Err(ProviderError::unavailable("mock unavailable", None));
            }
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(ProviderReply {
                    content: r#"{"narrative": "default mock response", "direction_change": false}"#
                        .to_string(),
                    similarity_score: Some(1.0),
                }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;

    #[derive(Clone)]
    struct MockProvider {
        state: Arc<Mutex<MockProviderState>>,
        gate: Option<Arc<ProviderGate>>,
    }

    #[derive(Clone, Debug)]
    struct MockProviderState {
        health: ProviderHealth,
        reply: Result<ProviderReply, ProviderError>,
        calls: usize,
        requests: Vec<ProviderRequest>,
    }

    struct ProviderGate {
        entered: Mutex<bool>,
        entered_cv: Condvar,
        released: Mutex<bool>,
        released_cv: Condvar,
    }

    impl ProviderGate {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                entered: Mutex::new(false),
                entered_cv: Condvar::new(),
                released: Mutex::new(false),
                released_cv: Condvar::new(),
            })
        }

        fn wait_until_entered(&self) {
            let mut entered = self.entered.lock().unwrap();
            while !*entered {
                entered = self.entered_cv.wait(entered).unwrap();
            }
        }

        fn release(&self) {
            let mut released = self.released.lock().unwrap();
            *released = true;
            self.released_cv.notify_all();
        }
    }

    impl MockProvider {
        fn available(reply: ProviderReply) -> (Self, Arc<Mutex<MockProviderState>>) {
            let state = Arc::new(Mutex::new(MockProviderState {
                health: ProviderHealth::available("ok"),
                reply: Ok(reply),
                calls: 0,
                requests: Vec::new(),
            }));

            (
                Self {
                    state: state.clone(),
                    gate: None,
                },
                state,
            )
        }

        fn unavailable(
            message: &str,
            setup_guide: Option<&str>,
        ) -> (Self, Arc<Mutex<MockProviderState>>) {
            let state = Arc::new(Mutex::new(MockProviderState {
                health: ProviderHealth::unavailable(message, setup_guide),
                reply: Err(ProviderError::unavailable(message, setup_guide)),
                calls: 0,
                requests: Vec::new(),
            }));

            (
                Self {
                    state: state.clone(),
                    gate: None,
                },
                state,
            )
        }

        fn failing(error: ProviderError) -> Self {
            let state = Arc::new(Mutex::new(MockProviderState {
                health: ProviderHealth::available("ok"),
                reply: Err(error),
                calls: 0,
                requests: Vec::new(),
            }));

            Self { state, gate: None }
        }

        fn blocking(reply: ProviderReply) -> (Self, Arc<ProviderGate>) {
            let gate = ProviderGate::new();
            let state = Arc::new(Mutex::new(MockProviderState {
                health: ProviderHealth::available("ok"),
                reply: Ok(reply),
                calls: 0,
                requests: Vec::new(),
            }));

            (
                Self {
                    state,
                    gate: Some(gate.clone()),
                },
                gate,
            )
        }
    }

    impl LlmProvider for MockProvider {
        fn health_check(&self) -> ProviderHealth {
            self.state.lock().unwrap().health.clone()
        }

        fn generate(&self, request: ProviderRequest) -> Result<ProviderReply, ProviderError> {
            if let Some(gate) = &self.gate {
                let mut entered = gate.entered.lock().unwrap();
                *entered = true;
                gate.entered_cv.notify_all();
                drop(entered);

                let mut released = gate.released.lock().unwrap();
                while !*released {
                    released = gate.released_cv.wait(released).unwrap();
                }
            }

            let mut state = self.state.lock().unwrap();
            state.calls += 1;
            state.requests.push(request);
            state.reply.clone()
        }
    }

    fn test_config(mode: LlmMode) -> LlmConfig {
        LlmConfig {
            mode,
            local_model: "llama3.1:8b".to_string(),
            cloud_model: Some("cloud-default".to_string()),
            direction_change_threshold: 0.55,
            max_retries: 5,
            initial_backoff_secs: 2,
        }
    }

    fn distill_request(context_id: &str) -> DistillationRequest {
        DistillationRequest {
            context_id: context_id.to_string(),
            raw_prompts: vec!["  keep exact spacing  ".to_string(), "推进中文".to_string()],
            previous_intent: Some("Keep moving forward".to_string()),
            language_hint: Some("zh-CN".to_string()),
        }
    }

    #[test]
    fn distill_routes_realtime_work_to_local_provider_in_hybrid_mode() {
        let (local, local_state) =
            MockProvider::available(ProviderReply::distillation("推进中文", Some(0.91)));
        let (cloud, cloud_state) = MockProvider::available(ProviderReply::compression("unused"));

        let engine = LlmEngine::new(
            test_config(LlmMode::Hybrid),
            Some(Box::new(local)),
            Some(Box::new(cloud)),
        );

        let outcome = engine
            .distill(distill_request("ctx-1"), 10)
            .expect("distillation should succeed");

        assert_eq!(outcome.intent.content, "推进中文");
        assert_eq!(outcome.provider, ProviderKind::Local);
        assert!(!outcome.intent.is_direction_change);

        let local_state = local_state.lock().unwrap();
        assert_eq!(local_state.calls, 1);
        assert_eq!(
            local_state.requests[0].raw_prompts[0],
            "  keep exact spacing  "
        );
        assert_eq!(local_state.requests[0].raw_prompts[1], "推进中文");
        assert_eq!(local_state.requests[0].language_hint.as_deref(), Some("zh-CN"));
        assert_eq!(local_state.requests[0].model, "llama3.1:8b");
        assert_eq!(outcome.intent.confidence, 0.91);
        assert_eq!(outcome.confidence, 0.91);

        let cloud_state = cloud_state.lock().unwrap();
        assert_eq!(cloud_state.calls, 0);
    }

    #[test]
    fn distill_marks_context_loading_while_provider_call_is_in_flight() {
        let (local, gate) =
            MockProvider::blocking(ProviderReply::distillation("working", Some(0.9)));
        let engine = Arc::new(LlmEngine::new(
            test_config(LlmMode::Local),
            Some(Box::new(local)),
            None,
        ));

        let engine_for_thread = engine.clone();
        let handle = thread::spawn(move || {
            engine_for_thread
                .distill(distill_request("ctx-loading"), 20)
                .expect("distillation should complete");
        });

        gate.wait_until_entered();
        let status = engine.get_llm_status().expect("status should be available");
        assert!(status.loading_contexts.contains(&"ctx-loading".to_string()));

        gate.release();
        handle.join().unwrap();

        let status = engine.get_llm_status().expect("status should be available");
        assert!(!status.loading_contexts.contains(&"ctx-loading".to_string()));
    }

    #[test]
    fn distill_queues_and_retries_with_exponential_backoff_up_to_five_attempts() {
        let (local, _) = MockProvider::unavailable(
            "ollama offline",
            Some("Start Ollama and install an 8B model."),
        );
        let engine = LlmEngine::new(test_config(LlmMode::Local), Some(Box::new(local)), None);

        let queued = engine
            .distill(distill_request("ctx-queue"), 100)
            .expect("initial queueing should succeed");

        assert_eq!(queued.disposition, DistillationDisposition::Queued);
        assert_eq!(queued.retry_after_secs, Some(102));

        let retried = engine
            .retry_queued_distillations(102)
            .expect("retry should run");
        assert_eq!(retried[0].attempt, 2);
        assert_eq!(retried[0].next_retry_at_secs, Some(106));

        let retried = engine
            .retry_queued_distillations(106)
            .expect("retry should run");
        assert_eq!(retried[0].attempt, 3);
        assert_eq!(retried[0].next_retry_at_secs, Some(114));

        let retried = engine
            .retry_queued_distillations(114)
            .expect("retry should run");
        assert_eq!(retried[0].attempt, 4);
        assert_eq!(retried[0].next_retry_at_secs, Some(130));

        let retried = engine
            .retry_queued_distillations(130)
            .expect("retry should run");
        assert_eq!(retried[0].attempt, 5);
        assert_eq!(retried[0].next_retry_at_secs, Some(162));

        let retried = engine
            .retry_queued_distillations(162)
            .expect("retry should run");
        assert_eq!(retried[0].attempt, 6);
        assert!(retried[0].next_retry_at_secs.is_none());

        let status = engine.get_llm_status().expect("status should be available");
        assert!(status.queue.is_empty());
    }

    #[test]
    fn distill_preserves_chinese_caps_narratives_and_flags_direction_change_in_one_call() {
        let long_text = "转".repeat(220);
        let (local, local_state) =
            MockProvider::available(ProviderReply::distillation(&long_text, Some(0.12)));
        let engine = LlmEngine::new(test_config(LlmMode::Local), Some(Box::new(local)), None);

        let outcome = engine
            .distill(distill_request("ctx-turn"), 50)
            .expect("distillation should succeed");

        assert_eq!(outcome.intent.content.chars().count(), 200);
        assert!(outcome.intent.content.chars().all(|ch| ch == '转'));
        assert_eq!(outcome.intent.confidence, 0.12);
        assert_eq!(outcome.confidence, 0.12);
        assert!(outcome.intent.is_direction_change);
        assert_eq!(outcome.intent.marker.as_deref(), Some("转折"));

        let local_state = local_state.lock().unwrap();
        assert_eq!(local_state.calls, 1);
    }

    #[test]
    fn compress_batch_uses_cloud_for_hybrid_summary_creation_and_archives_sources() {
        let (local, local_state) = MockProvider::available(ProviderReply::compression("unused"));
        let (cloud, cloud_state) = MockProvider::available(ProviderReply::compression("summary"));
        let engine = LlmEngine::new(
            test_config(LlmMode::Hybrid),
            Some(Box::new(local)),
            Some(Box::new(cloud)),
        );

        let outcome = engine
            .compress_batch(
                CompressionRequest {
                    context_id: "ctx-summary".to_string(),
                    intents: vec![
                        IntentRecord::narrative("n-1", "first", 0),
                        IntentRecord::narrative("n-2", "second", 1),
                    ],
                    target_tier: IntentTier::Summary,
                },
                14_401,
            )
            .expect("compression should succeed")
            .expect("compression should happen");

        assert_eq!(outcome.provider, ProviderKind::Cloud);
        assert_eq!(outcome.compressed_intent.tier, IntentTier::Summary);
        assert_eq!(
            outcome.compressed_intent.compressed_from,
            vec!["n-1", "n-2"]
        );
        assert!(outcome
            .archived_sources
            .iter()
            .all(|intent| intent.archived));

        let local_state = local_state.lock().unwrap();
        assert_eq!(local_state.calls, 0);

        let cloud_state = cloud_state.lock().unwrap();
        assert_eq!(cloud_state.calls, 1);
        assert_eq!(cloud_state.requests[0].model, "cloud-default");
    }

    #[test]
    fn compress_batch_uses_local_for_label_creation_in_local_mode() {
        let (local, local_state) = MockProvider::available(ProviderReply::compression("label"));
        let engine = LlmEngine::new(test_config(LlmMode::Local), Some(Box::new(local)), None);

        let outcome = engine
            .compress_batch(
                CompressionRequest {
                    context_id: "ctx-label".to_string(),
                    intents: vec![
                        IntentRecord::summary("s-1", "summary one", 0),
                        IntentRecord::summary("s-2", "summary two", 10),
                    ],
                    target_tier: IntentTier::Label,
                },
                259_211,
            )
            .expect("compression should succeed")
            .expect("compression should happen");

        assert_eq!(outcome.provider, ProviderKind::Local);
        assert_eq!(outcome.compressed_intent.tier, IntentTier::Label);

        let local_state = local_state.lock().unwrap();
        assert_eq!(local_state.calls, 1);
        assert_eq!(local_state.requests[0].model, "llama3.1:8b");
    }

    #[test]
    fn health_check_reports_setup_guide_and_cloud_fallback_when_local_is_missing() {
        let (local, _) = MockProvider::unavailable(
            "ollama not installed",
            Some("Install Ollama, then pull an 8B model."),
        );
        let (cloud, _) = MockProvider::available(ProviderReply::compression("ok"));

        let engine = LlmEngine::new(
            test_config(LlmMode::Hybrid),
            Some(Box::new(local)),
            Some(Box::new(cloud)),
        );

        let report = engine.health_check().expect("health check should succeed");

        assert!(!report.local.available);
        assert!(report.cloud.available);
        assert!(report.degraded_to_cloud_only);
        assert_eq!(
            report.setup_guide.as_deref(),
            Some("Install Ollama, then pull an 8B model.")
        );
    }

    #[test]
    fn distill_maps_timeout_and_parse_errors_to_spec_variants() {
        let timeout_engine = LlmEngine::new(
            test_config(LlmMode::Local),
            Some(Box::new(MockProvider::failing(ProviderError::Timeout))),
            None,
        );

        let timeout_error = timeout_engine
            .distill(distill_request("ctx-timeout"), 10)
            .expect_err("timeout should bubble up");
        assert_eq!(
            timeout_error,
            LlmError::Timeout {
                provider: ProviderKind::Local,
            }
        );

        let parse_engine = LlmEngine::new(
            test_config(LlmMode::Local),
            Some(Box::new(MockProvider::failing(
                ProviderError::ResponseParseError("bad json".to_string()),
            ))),
            None,
        );

        let parse_error = parse_engine
            .distill(distill_request("ctx-parse"), 10)
            .expect_err("parse failure should bubble up");
        assert_eq!(
            parse_error,
            LlmError::ResponseParseError {
                provider: ProviderKind::Local,
                message: "bad json".to_string(),
            }
        );
    }

    #[test]
    fn compress_batch_rejects_mixed_tier_batches() {
        let (local, local_state) = MockProvider::available(ProviderReply::compression("unused"));
        let engine = LlmEngine::new(test_config(LlmMode::Local), Some(Box::new(local)), None);

        let error = engine
            .compress_batch(
                CompressionRequest {
                    context_id: "ctx-mixed".to_string(),
                    intents: vec![
                        IntentRecord::narrative("n-1", "first", 0),
                        IntentRecord::summary("s-1", "second", 1),
                    ],
                    target_tier: IntentTier::Summary,
                },
                50,
            )
            .expect_err("mixed tiers should be rejected");

        assert_eq!(
            error,
            LlmError::InvalidRequest(
                "compression batch must contain intents from a single tier".to_string()
            )
        );
        assert_eq!(local_state.lock().unwrap().calls, 0);
    }
}
