/// Architecture Boundary Test — Backend (Rust)
///
/// Validates that internal crate modules only import from their permitted layers.
/// Layer rules: docs/architecture/LAYERS.md
///
/// Violation format:
///   VIOLATION: src/{module}/mod.rs:{line} uses crate::{target}
///   — {module} cannot import {target}. See docs/architecture/LAYERS.md
use regex::Regex;
use std::fs;

/// Returns which internal crate:: modules each module is allowed to import.
/// lib.rs is the composition root and is exempt — not checked here.
fn allowed_internal_imports(module: &str) -> Vec<&'static str> {
    match module {
        "commands" => vec!["db", "events"],
        "capture" => vec![],  // leaf — external crates only
        "watcher" => vec![],  // leaf — external crates only
        "intent" => vec![],   // leaf
        "events" => vec![],   // leaf
        "db" => vec![],       // leaf
        _ => vec![],
    }
}

fn scan_module(module: &str) -> Vec<String> {
    let path = format!("src/{}/mod.rs", module);
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let re = Regex::new(r"use crate::(\w+)").expect("invalid regex");
    let allowed = allowed_internal_imports(module);
    let mut violations = vec![];

    for (line_idx, line) in content.lines().enumerate() {
        for cap in re.captures_iter(line) {
            let target = &cap[1];
            // Skip self-references
            if target == module {
                continue;
            }
            if !allowed.contains(&target) {
                violations.push(format!(
                    "VIOLATION: src/{}/mod.rs:{} uses crate::{} — {} cannot import {}. See docs/architecture/LAYERS.md",
                    module,
                    line_idx + 1,
                    target,
                    module,
                    target
                ));
            }
        }
    }
    violations
}

#[test]
fn test_architecture_boundaries() {
    let modules = ["commands", "capture", "watcher", "intent", "events", "db"];
    let mut all_violations: Vec<String> = vec![];

    for module in &modules {
        all_violations.extend(scan_module(module));
    }

    assert!(
        all_violations.is_empty(),
        "Architecture boundary violations found:\n{}\n\nSee docs/architecture/LAYERS.md for the rules.",
        all_violations.join("\n")
    );
}
