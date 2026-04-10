use serde::{Deserialize, Serialize};

/// Context status states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextStatus {
    Running,
    Done,
    Stuck,
    Parked,
}

impl ContextStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextStatus::Running => "running",
            ContextStatus::Done => "done",
            ContextStatus::Stuck => "stuck",
            ContextStatus::Parked => "parked",
        }
    }

    pub fn parse(s: &str) -> Result<Self, &'static str> {
        match s {
            "running" => Ok(ContextStatus::Running),
            "done" => Ok(ContextStatus::Done),
            "stuck" => Ok(ContextStatus::Stuck),
            "parked" => Ok(ContextStatus::Parked),
            _ => Err("invalid status"),
        }
    }
}

/// Signal types that trigger status transitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalType {
    NewSessionDetected,
    GitCommitOrPush,
    ErrorPattern10Min,
    NoActivity30Min,
    NewPromptsDetected,
    NewNonErrorPrompts,
    ManualOverride,
}

impl SignalType {
    pub fn parse(s: &str) -> Result<Self, &'static str> {
        match s {
            "new_session_detected" => Ok(SignalType::NewSessionDetected),
            "git_commit_or_push" => Ok(SignalType::GitCommitOrPush),
            "error_pattern_10min" => Ok(SignalType::ErrorPattern10Min),
            "no_activity_30min" => Ok(SignalType::NoActivity30Min),
            "new_prompts_detected" => Ok(SignalType::NewPromptsDetected),
            "new_non_error_prompts" => Ok(SignalType::NewNonErrorPrompts),
            "manual_override" => Ok(SignalType::ManualOverride),
            _ => Err("invalid signal type"),
        }
    }
}

/// Attempt a status transition. Returns the new status if the transition is valid,
/// or None if the transition is not allowed.
pub fn try_transition(current: &ContextStatus, signal: &SignalType) -> Option<ContextStatus> {
    match (current, signal) {
        // parked → running: new_session_detected
        (ContextStatus::Parked, SignalType::NewSessionDetected) => Some(ContextStatus::Running),

        // running → done: git_commit_or_push
        (ContextStatus::Running, SignalType::GitCommitOrPush) => Some(ContextStatus::Done),

        // running → stuck: error_pattern_10min
        (ContextStatus::Running, SignalType::ErrorPattern10Min) => Some(ContextStatus::Stuck),

        // running → parked: no_activity_30min
        (ContextStatus::Running, SignalType::NoActivity30Min) => Some(ContextStatus::Parked),

        // done → running: new_prompts_detected
        (ContextStatus::Done, SignalType::NewPromptsDetected) => Some(ContextStatus::Running),

        // stuck → running: new_non_error_prompts
        (ContextStatus::Stuck, SignalType::NewNonErrorPrompts) => Some(ContextStatus::Running),

        // done/stuck → parked: no_activity_30min
        (ContextStatus::Done, SignalType::NoActivity30Min) => Some(ContextStatus::Parked),
        (ContextStatus::Stuck, SignalType::NoActivity30Min) => Some(ContextStatus::Parked),

        // Manual override: any → any (caller must enforce cooldown)
        (_, SignalType::ManualOverride) => None, // handled externally with target status

        // All other transitions are invalid
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parked_to_running() {
        let result = try_transition(&ContextStatus::Parked, &SignalType::NewSessionDetected);
        assert_eq!(result, Some(ContextStatus::Running));
    }

    #[test]
    fn test_running_to_done() {
        let result = try_transition(&ContextStatus::Running, &SignalType::GitCommitOrPush);
        assert_eq!(result, Some(ContextStatus::Done));
    }

    #[test]
    fn test_running_to_stuck() {
        let result = try_transition(&ContextStatus::Running, &SignalType::ErrorPattern10Min);
        assert_eq!(result, Some(ContextStatus::Stuck));
    }

    #[test]
    fn test_running_to_parked() {
        let result = try_transition(&ContextStatus::Running, &SignalType::NoActivity30Min);
        assert_eq!(result, Some(ContextStatus::Parked));
    }

    #[test]
    fn test_done_to_running() {
        let result = try_transition(&ContextStatus::Done, &SignalType::NewPromptsDetected);
        assert_eq!(result, Some(ContextStatus::Running));
    }

    #[test]
    fn test_stuck_to_running() {
        let result = try_transition(&ContextStatus::Stuck, &SignalType::NewNonErrorPrompts);
        assert_eq!(result, Some(ContextStatus::Running));
    }

    #[test]
    fn test_done_to_parked() {
        let result = try_transition(&ContextStatus::Done, &SignalType::NoActivity30Min);
        assert_eq!(result, Some(ContextStatus::Parked));
    }

    #[test]
    fn test_stuck_to_parked() {
        let result = try_transition(&ContextStatus::Stuck, &SignalType::NoActivity30Min);
        assert_eq!(result, Some(ContextStatus::Parked));
    }

    #[test]
    fn test_invalid_transition() {
        // parked cannot go to done directly
        let result = try_transition(&ContextStatus::Parked, &SignalType::GitCommitOrPush);
        assert_eq!(result, None);
    }

    #[test]
    fn test_status_roundtrip() {
        for status in &[ContextStatus::Running, ContextStatus::Done, ContextStatus::Stuck, ContextStatus::Parked] {
            let s = status.as_str();
            let parsed = ContextStatus::parse(s).unwrap();
            assert_eq!(&parsed, status);
        }
    }
}
