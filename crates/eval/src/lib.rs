//! Eval case format and replay.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("load error: {0}")]
    Load(String),
    #[error("validation: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub input: Vec<Value>, // messages as JSON
    #[serde(default)]
    pub expected_tool_calls: Option<Vec<String>>,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    #[serde(default)]
    pub rubric: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Assertion {
    Contains { value: String },
    NotContains { value: String },
    Regex { pattern: String },
    JsonSchema { schema: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub case_id: String,
    pub passed: bool,
    pub tool_calls: Vec<String>,
    pub output: String,
    pub tokens: u32,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub assertion_failures: Vec<String>,
    pub judge_verdict: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    pub cases: Vec<EvalResult>,
    pub pass_count: usize,
    pub fail_count: usize,
    pub total_cost_usd: f64,
    pub total_latency_ms: u64,
}

pub fn load_cases(path: &str) -> Result<Vec<EvalCase>, EvalError> {
    let content = std::fs::read_to_string(path).map_err(|e| EvalError::Load(e.to_string()))?;
    let mut cases = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let c: EvalCase = serde_json::from_str(line).map_err(|e| EvalError::Load(format!("line {}: {}", i + 1, e)))?;
        validate_case(&c)?;
        cases.push(c);
    }
    Ok(cases)
}

fn validate_case(c: &EvalCase) -> Result<(), EvalError> {
    if c.id.trim().is_empty() {
        return Err(EvalError::Validation("id must not be empty".into()));
    }
    if c.input.is_empty() {
        return Err(EvalError::Validation(format!("case {}: input empty", c.id)));
    }
    Ok(())
}

/// Mock tool mode flag.
#[derive(Debug, Clone, Copy)]
pub enum Mode { Mock, Live }

pub async fn run_case(
    _recipe_path: &str,
    case: &EvalCase,
    _mode: Mode,
    _workspace: &str,
) -> Result<EvalResult, EvalError> {
    // Replay through core would happen here; isolated workspace + mock tools.
    // Stub returning a placeholder result (no execution).
    Ok(EvalResult {
        case_id: case.id.clone(),
        passed: true,
        tool_calls: Vec::new(),
        output: String::new(),
        tokens: 0,
        cost_usd: 0.0,
        latency_ms: 0,
        assertion_failures: Vec::new(),
        judge_verdict: None,
    })
}

pub fn compare_reports(a: &RegressionReport, b: &RegressionReport) -> serde_json::Value {
    serde_json::json!({
        "a_pass": a.pass_count,
        "b_pass": b.pass_count,
        "delta_pass": b.pass_count as i64 - a.pass_count as i64,
    })
}
EOF