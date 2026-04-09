use crate::db::{Database, IntentRecord};
use crate::llm::{LlmRouter, ProviderRequest, Workload};
use super::errors::BrainError;
use std::collections::HashMap;
use std::sync::Arc;

/// Run a compression cycle: compress stale narratives→summaries and stale summaries→labels.
/// Returns the number of compressed intents created.
pub fn run_compression_cycle(
    db: &Arc<Database>,
    llm: &Arc<LlmRouter>,
) -> Result<i32, BrainError> {
    let reference_time = chrono::Utc::now().to_rfc3339();
    let stale = db
        .get_stale_intents(&reference_time)
        .map_err(BrainError::DbError)?;

    if stale.is_empty() {
        return Ok(0);
    }

    // Group by context_id and tier
    let mut narratives_by_ctx: HashMap<String, Vec<IntentRecord>> = HashMap::new();
    let mut summaries_by_ctx: HashMap<String, Vec<IntentRecord>> = HashMap::new();

    for intent in stale {
        match intent.tier.as_str() {
            "narrative" => narratives_by_ctx
                .entry(intent.context_id.clone())
                .or_default()
                .push(intent),
            "summary" => summaries_by_ctx
                .entry(intent.context_id.clone())
                .or_default()
                .push(intent),
            _ => {}
        }
    }

    let mut compressed_count = 0;

    // Compress narratives → summary
    for (context_id, narratives) in &narratives_by_ctx {
        if narratives.len() < 2 {
            continue; // Need at least 2 to compress
        }

        let compressed = compress_group(db, llm, context_id, narratives, "summary")?;
        if compressed {
            compressed_count += 1;
        }
    }

    // Compress summaries → label
    for (context_id, summaries) in &summaries_by_ctx {
        if summaries.len() < 2 {
            continue;
        }

        let compressed = compress_group(db, llm, context_id, summaries, "label")?;
        if compressed {
            compressed_count += 1;
        }
    }

    Ok(compressed_count)
}

fn compress_group(
    db: &Arc<Database>,
    llm: &Arc<LlmRouter>,
    context_id: &str,
    intents: &[IntentRecord],
    target_tier: &str,
) -> Result<bool, BrainError> {
    let user_prompt = build_compress_prompt(intents, target_tier);
    let system_prompt = match target_tier {
        "summary" => "Compress these narrative-tier intents into a single summary (max 500 chars) that captures the overall trajectory of work.".to_string(),
        "label" => "Compress these summary-tier intents into a single label (max 100 chars) that captures the essence of this work stream.".to_string(),
        _ => return Err(BrainError::InvalidStatus(format!("Unknown target tier: {target_tier}"))),
    };

    let request = ProviderRequest {
        system_prompt,
        user_prompt,
        max_tokens: 512,
        temperature: 0.3,
    };

    let reply = llm
        .route(Workload::Compression, request)
        .map_err(|e| BrainError::LlmUnavailable(e.to_string()))?;

    // Store source IDs as JSON array
    let source_ids: Vec<String> = intents.iter().map(|i| i.id.clone()).collect();
    let compressed_from = serde_json::to_string(&source_ids)
        .map_err(|e| BrainError::DbError(e.to_string()))?;

    // Create compressed intent
    db.insert_intent_v2(
        context_id,
        target_tier,
        &reply.content,
        "compression",
        Some(&compressed_from),
    )
    .map_err(BrainError::DbError)?;

    // Archive source intents
    db.archive_intents(&source_ids)
        .map_err(BrainError::DbError)?;

    Ok(true)
}

fn build_compress_prompt(intents: &[IntentRecord], target_tier: &str) -> String {
    let mut parts = Vec::new();
    for (i, intent) in intents.iter().enumerate() {
        parts.push(format!(
            "{}. [{}] {}",
            i + 1,
            intent.tier,
            intent.content
        ));
    }
    format!(
        "Compress the following {} intents into a single {} intent:\n\n{}",
        intents.len(),
        target_tier,
        parts.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_compress_prompt() {
        let intents = vec![
            IntentRecord {
                id: "1".to_string(),
                context_id: "ctx".to_string(),
                tier: "narrative".to_string(),
                content: "Fixed login bug".to_string(),
                source: "auto".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                archived: false,
                archived_at: None,
                compressed_from: None,
            },
            IntentRecord {
                id: "2".to_string(),
                context_id: "ctx".to_string(),
                tier: "narrative".to_string(),
                content: "Added auth tests".to_string(),
                source: "auto".to_string(),
                created_at: "2024-01-01T01:00:00Z".to_string(),
                archived: false,
                archived_at: None,
                compressed_from: None,
            },
        ];

        let prompt = build_compress_prompt(&intents, "summary");
        assert!(prompt.contains("1. [narrative] Fixed login bug"));
        assert!(prompt.contains("2. [narrative] Added auth tests"));
        assert!(prompt.contains("summary"));
    }
}
