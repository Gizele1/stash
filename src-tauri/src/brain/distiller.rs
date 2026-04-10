use crate::db::{Database, IntentRecord, RawPromptRecord};
use crate::llm::{LlmRouter, ProviderRequest, Workload};
use super::errors::BrainError;
use std::sync::Arc;

/// Distillation threshold: trigger when pending prompts reach this count
pub const DISTILL_THRESHOLD: i64 = 3;
/// Maximum pending prompts to process in one batch
pub const DISTILL_WINDOW: i64 = 5;
/// Maximum characters for a narrative intent
pub const NARRATIVE_MAX_CHARS: usize = 200;

/// Result of a distillation: the new intent (if created) and whether it's a direction change
pub struct DistillResult {
    pub intent: Option<IntentRecord>,
    pub direction_change: bool,
}

/// Attempt distillation for a context if enough pending prompts exist.
pub fn maybe_distill(
    db: &Arc<Database>,
    llm: &Arc<LlmRouter>,
    context_id: &str,
) -> Result<DistillResult, BrainError> {
    let pending = db
        .get_pending_prompts(context_id, DISTILL_WINDOW)
        .map_err(BrainError::DbError)?;

    if (pending.len() as i64) < DISTILL_THRESHOLD {
        return Ok(DistillResult {
            intent: None,
            direction_change: false,
        });
    }

    // Build prompt from pending raw prompts
    let user_prompt = build_distill_prompt(&pending);
    let system_prompt = "You are a thought distiller. Given a sequence of AI coding prompts, produce a concise narrative (max 200 chars) that captures the developer's current intent. Also determine if there's a direction change from previous work.\n\nRespond in JSON: {\"narrative\": \"...\", \"direction_change\": true/false}".to_string();

    let request = ProviderRequest {
        system_prompt,
        user_prompt,
        max_tokens: 256,
        temperature: 0.3,
    };

    let reply = llm
        .route(Workload::Distillation, request)
        .map_err(|e| BrainError::LlmUnavailable(e.to_string()))?;

    // Parse response
    let (narrative, direction_change) = parse_distill_response(&reply.content)?;

    // Truncate to max chars
    let narrative = if narrative.chars().count() > NARRATIVE_MAX_CHARS {
        narrative.chars().take(NARRATIVE_MAX_CHARS).collect()
    } else {
        narrative
    };

    // Create intent
    let intent = db
        .insert_intent_v2(context_id, "narrative", &narrative, "auto", None)
        .map_err(BrainError::DbError)?;

    // Mark prompts as consumed
    let prompt_ids: Vec<String> = pending.iter().map(|p| p.id.clone()).collect();
    db.mark_consumed(&prompt_ids).map_err(BrainError::DbError)?;

    Ok(DistillResult {
        intent: Some(intent),
        direction_change,
    })
}

fn build_distill_prompt(prompts: &[RawPromptRecord]) -> String {
    let mut parts = Vec::new();
    for (i, p) in prompts.iter().enumerate() {
        parts.push(format!(
            "Prompt {} [{}]: {}",
            i + 1,
            p.role,
            // Truncate very long prompts for the LLM
            if p.content.len() > 500 {
                format!("{}...", &p.content[..500])
            } else {
                p.content.clone()
            }
        ));
    }
    parts.join("\n\n")
}

fn parse_distill_response(content: &str) -> Result<(String, bool), BrainError> {
    // Try to parse as JSON
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
        let narrative = v
            .get("narrative")
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown intent")
            .to_string();
        let direction_change = v
            .get("direction_change")
            .and_then(|d| d.as_bool())
            .unwrap_or(false);
        return Ok((narrative, direction_change));
    }

    // Fallback: use the raw content as narrative
    let truncated = if content.chars().count() > NARRATIVE_MAX_CHARS {
        content.chars().take(NARRATIVE_MAX_CHARS).collect()
    } else {
        content.to_string()
    };
    Ok((truncated, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_distill_response_json() {
        let json = r#"{"narrative": "Refactoring auth module for better error handling", "direction_change": false}"#;
        let (narrative, dc) = parse_distill_response(json).unwrap();
        assert_eq!(narrative, "Refactoring auth module for better error handling");
        assert!(!dc);
    }

    #[test]
    fn test_parse_distill_response_direction_change() {
        let json = r#"{"narrative": "Switching to new API design", "direction_change": true}"#;
        let (narrative, dc) = parse_distill_response(json).unwrap();
        assert_eq!(narrative, "Switching to new API design");
        assert!(dc);
    }

    #[test]
    fn test_parse_distill_response_fallback() {
        let content = "Just a plain text response";
        let (narrative, dc) = parse_distill_response(content).unwrap();
        assert_eq!(narrative, "Just a plain text response");
        assert!(!dc);
    }

    #[test]
    fn test_build_distill_prompt() {
        let prompts = vec![
            RawPromptRecord {
                id: "1".to_string(),
                context_id: "ctx".to_string(),
                session_path: "/s".to_string(),
                message_id: "m1".to_string(),
                role: "user".to_string(),
                content: "Fix the login bug".to_string(),
                captured_at: "2024-01-01T00:00:00Z".to_string(),
            },
            RawPromptRecord {
                id: "2".to_string(),
                context_id: "ctx".to_string(),
                session_path: "/s".to_string(),
                message_id: "m2".to_string(),
                role: "assistant".to_string(),
                content: "I'll fix the auth module".to_string(),
                captured_at: "2024-01-01T00:01:00Z".to_string(),
            },
        ];
        let result = build_distill_prompt(&prompts);
        assert!(result.contains("Prompt 1 [user]: Fix the login bug"));
        assert!(result.contains("Prompt 2 [assistant]: I'll fix the auth module"));
    }
}
