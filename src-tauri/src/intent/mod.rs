use regex::Regex;

/// Strategy trait for intent extraction — allows swapping SimpleRuleExtractor for LLMExtractor later.
pub trait IntentExtractor: Send + Sync {
    fn extract(&self, input: &str) -> Option<String>;
}

/// Truncate a string to at most `max` bytes, respecting UTF-8 char boundaries.
fn truncate_safe(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// MVP extractor using regex patterns to pull intent from agent prompts/messages.
pub struct SimpleRuleExtractor {
    patterns: Vec<Regex>,
}

impl SimpleRuleExtractor {
    pub fn new() -> Self {
        let patterns = vec![
            Regex::new(r"(?i)(?:implement|build|create|add|fix|refactor|update|migrate)\s+(.+)")
                .unwrap(),
            Regex::new(r"(?i)(?:working on|task:|goal:|intent:)\s*(.+)").unwrap(),
            Regex::new(r"(?i)(?:please|can you|could you)\s+(.+)").unwrap(),
        ];
        Self { patterns }
    }

    /// Try to extract a concise intent statement from raw text.
    pub fn extract_intent(&self, input: &str) -> Option<String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        for pattern in &self.patterns {
            if let Some(caps) = pattern.captures(trimmed) {
                if let Some(m) = caps.get(1) {
                    let extracted = m.as_str().trim();
                    // Cap at first sentence or 120 chars
                    let statement = extracted
                        .split_once('.')
                        .map(|(s, _)| s)
                        .unwrap_or(extracted);
                    let statement = truncate_safe(statement, 120);
                    return Some(statement.to_string());
                }
            }
        }

        // Fallback: use first line, capped
        let first_line = trimmed.lines().next().unwrap_or(trimmed);
        Some(truncate_safe(first_line, 120).to_string())
    }
}

impl IntentExtractor for SimpleRuleExtractor {
    fn extract(&self, input: &str) -> Option<String> {
        self.extract_intent(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_implement_pattern() {
        let extractor = SimpleRuleExtractor::new();
        let result = extractor.extract_intent("implement user authentication with JWT");
        assert_eq!(result, Some("user authentication with JWT".to_string()));
    }

    #[test]
    fn test_extract_working_on_pattern() {
        let extractor = SimpleRuleExtractor::new();
        let result = extractor.extract_intent("working on fixing the login bug");
        assert_eq!(result, Some("fixing the login bug".to_string()));
    }

    #[test]
    fn test_extract_empty_input() {
        let extractor = SimpleRuleExtractor::new();
        assert_eq!(extractor.extract_intent(""), None);
        assert_eq!(extractor.extract_intent("   "), None);
    }

    #[test]
    fn test_fallback_first_line() {
        let extractor = SimpleRuleExtractor::new();
        let result = extractor.extract_intent("some random text\nsecond line");
        assert_eq!(result, Some("some random text".to_string()));
    }
}
