pub mod state_machine;
pub mod distiller;
pub mod compressor;
pub mod errors;

#[cfg(test)]
mod tests;

use crate::db::{ContextRecord, Database, IntentRecord};
use crate::llm::LlmRouter;
use errors::BrainError;
use state_machine::{ContextStatus, SignalType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Maximum number of active (non-parked) contexts
const MAX_ACTIVE_CONTEXTS: i64 = 4;
/// Manual override cooldown in minutes
const OVERRIDE_COOLDOWN_MINUTES: i64 = 15;

// ── Public API types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonlMessage {
    pub project_hash: String,
    pub session_id: String,
    pub project_dir: String,
    pub display_name: String,
    pub message_id: String,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWithStatus {
    pub id: String,
    pub project_key: String,
    pub project_dir: String,
    pub name: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDetail {
    pub context: ContextRecord,
    pub current_intent: Option<IntentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentTimeline {
    pub intents: Vec<IntentRecord>,
    pub has_more: bool,
    pub hidden_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
}

// ── Brain struct ──

pub struct Brain {
    pub(crate) db: Arc<Database>,
    llm: Arc<LlmRouter>,
}

impl Brain {
    pub fn new(db: Arc<Database>, llm: Arc<LlmRouter>) -> Self {
        Self { db, llm }
    }

    // ── Core internal functions ──

    /// Handle an incoming raw prompt message.
    /// Find or create context, store as RawPrompt, maybe trigger distillation.
    pub fn handle_raw_prompt(&self, message: JsonlMessage) -> Result<(String, String), BrainError> {
        // Check max active contexts before potentially creating a new one
        let active_count = self.db.count_active_contexts().map_err(BrainError::DbError)?;

        // Check if context already exists
        let existing = self.db
            .get_context_by_project_dir(&message.project_dir)
            .map_err(BrainError::DbError)?;

        if existing.is_none() && active_count >= MAX_ACTIVE_CONTEXTS {
            return Err(BrainError::MaxContextsReached);
        }

        // Upsert context
        let context = self.db
            .upsert_context(&message.project_hash, Some(&message.project_dir), Some(&message.display_name))
            .map_err(BrainError::DbError)?;

        // Store raw prompt
        let prompt = self.db
            .insert_raw_prompt(
                &context.id,
                &message.session_id,
                &message.message_id,
                &message.role,
                &message.content,
            )
            .map_err(BrainError::DbError)?;

        Ok((context.id, prompt.id))
    }

    /// Handle a git signal (commit, push, error, inactivity).
    pub fn handle_git_signal(
        &self,
        project_dir: &str,
        signal_type: &str,
        _metadata: Option<&str>,
    ) -> Result<(String, String), BrainError> {
        let context = self.db
            .get_context_by_project_dir(project_dir)
            .map_err(BrainError::DbError)?
            .ok_or_else(|| BrainError::ContextNotFound(project_dir.to_string()))?;

        let current_status = ContextStatus::parse(&context.status)
            .map_err(|e| BrainError::InvalidStatus(e.to_string()))?;

        let signal = SignalType::parse(signal_type)
            .map_err(|e| BrainError::InvalidStatus(e.to_string()))?;

        // Check if override is active
        if let Some(ref override_until) = context.status_override_until {
            if let Ok(until) = chrono::DateTime::parse_from_rfc3339(override_until) {
                if chrono::Utc::now() < until {
                    // Override is active, don't auto-transition
                    return Ok((context.id, context.status));
                }
            }
        }

        let new_status = state_machine::try_transition(&current_status, &signal);

        match new_status {
            Some(ns) => {
                let updated = self.db
                    .update_context_status(&context.id, ns.as_str(), None)
                    .map_err(BrainError::DbError)?;
                Ok((updated.id, updated.status))
            }
            None => {
                // No valid transition
                Ok((context.id, context.status))
            }
        }
    }

    /// Attempt distillation for a context.
    pub fn maybe_distill(
        &self,
        context_id: &str,
    ) -> Result<(Option<IntentRecord>, bool), BrainError> {
        // Verify context exists
        self.db
            .get_context_by_id(context_id)
            .map_err(BrainError::DbError)?;

        let result = distiller::maybe_distill(&self.db, &self.llm, context_id)?;
        Ok((result.intent, result.direction_change))
    }

    /// Run a compression cycle across all contexts.
    pub fn run_compression_cycle(&self) -> Result<i32, BrainError> {
        compressor::run_compression_cycle(&self.db, &self.llm)
    }

    // ── Tauri API functions ──

    /// List all active contexts with their current status.
    pub fn get_contexts(&self) -> Result<Vec<ContextWithStatus>, BrainError> {
        let contexts = self.db
            .list_active_contexts()
            .map_err(BrainError::DbError)?;

        Ok(contexts
            .into_iter()
            .map(|c| ContextWithStatus {
                id: c.id,
                project_key: c.project_key,
                project_dir: c.project_dir.unwrap_or_default(),
                name: c.name,
                status: c.status,
                updated_at: c.updated_at,
            })
            .collect())
    }

    /// Get full detail for a context including current intent.
    pub fn get_context_detail(&self, context_id: &str) -> Result<ContextDetail, BrainError> {
        let context = self.db
            .get_context_by_id(context_id)
            .map_err(BrainError::DbError)?;

        let current_intent = self.db
            .get_latest_intent_for_context(context_id)
            .map_err(BrainError::DbError)?;

        Ok(ContextDetail {
            context,
            current_intent,
        })
    }

    /// Get paginated intent timeline for a context.
    pub fn get_intent_timeline(
        &self,
        context_id: &str,
        limit: i64,
        before_id: Option<&str>,
    ) -> Result<IntentTimeline, BrainError> {
        // Fetch one extra to determine has_more
        let intents = self.db
            .get_intents_for_context(context_id, limit + 1, before_id)
            .map_err(BrainError::DbError)?;

        let has_more = intents.len() as i64 > limit;
        let intents = if has_more {
            intents.into_iter().take(limit as usize).collect()
        } else {
            intents
        };

        let hidden_count = self.db
            .count_archived_intents(context_id)
            .map_err(BrainError::DbError)?;

        Ok(IntentTimeline {
            intents,
            has_more,
            hidden_count,
        })
    }

    /// Expand a compressed intent to show its source intents.
    pub fn expand_compressed_intent(
        &self,
        intent_id: &str,
    ) -> Result<Vec<IntentRecord>, BrainError> {
        let intent = self.db
            .get_intent_v2(intent_id)
            .map_err(BrainError::DbError)?;

        if intent.compressed_from.is_none() {
            return Err(BrainError::NotCompressed);
        }

        self.db
            .get_intents_compressed_from(intent_id)
            .map_err(BrainError::DbError)
    }

    /// Manual status override with 15-minute cooldown.
    pub fn override_status(
        &self,
        context_id: &str,
        new_status: &str,
    ) -> Result<bool, BrainError> {
        let context = self.db
            .get_context_by_id(context_id)
            .map_err(BrainError::DbError)?;

        // Check cooldown
        if let Some(ref override_until) = context.status_override_until {
            if let Ok(until) = chrono::DateTime::parse_from_rfc3339(override_until) {
                if chrono::Utc::now() < until {
                    return Ok(false); // Cooldown active
                }
            }
        }

        // Validate target status
        let _ = ContextStatus::parse(new_status)
            .map_err(|_| BrainError::InvalidStatus(new_status.to_string()))?;

        // Set override with cooldown
        let override_until = (chrono::Utc::now()
            + chrono::Duration::minutes(OVERRIDE_COOLDOWN_MINUTES))
        .to_rfc3339();

        self.db
            .update_context_status(context_id, new_status, Some(&override_until))
            .map_err(BrainError::DbError)?;

        Ok(true)
    }

    /// Submit a manual intent. If context_id is None, uses the most recently active context.
    pub fn submit_manual_intent(
        &self,
        context_id: Option<&str>,
        content: &str,
    ) -> Result<String, BrainError> {
        let resolved_id = match context_id {
            Some(id) => {
                self.db.get_context_by_id(id).map_err(BrainError::DbError)?;
                id.to_string()
            }
            None => {
                let active = self.db.list_active_contexts().map_err(BrainError::DbError)?;
                active
                    .first()
                    .ok_or(BrainError::NoActiveContext)?
                    .id
                    .clone()
            }
        };

        let intent = self.db
            .insert_intent_v2(&resolved_id, "narrative", content, "manual", None)
            .map_err(BrainError::DbError)?;

        Ok(intent.id)
    }

    /// Correct an existing intent by creating a correction record.
    pub fn correct_intent(
        &self,
        intent_id: &str,
        new_content: &str,
    ) -> Result<String, BrainError> {
        let original = self.db
            .get_intent_v2(intent_id)
            .map_err(BrainError::DbError)?;

        let correction = self.db
            .insert_intent_v2(
                &original.context_id,
                &original.tier,
                new_content,
                "manual_correction",
                None,
            )
            .map_err(BrainError::DbError)?;

        Ok(correction.id)
    }

    pub fn llm_status(&self) -> serde_json::Value {
        let health = self.llm.provider_health();
        let config = self.llm.config();
        serde_json::json!({
            "mode": format!("{:?}", config.mode),
            "ollama_ok": health.available,
            "cloud_ok": false,
        })
    }
}
