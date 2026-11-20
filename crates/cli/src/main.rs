//! Headless CLI for the Kestrel agent harness.

use anyhow::Result;
use clap::Parser;
use kestrel_core::{run_agent, AgentConfig};
use kestrel_modules::{BashTool, EditTool, OpenAiProvider, ReadTool, SkillTool, ListSkillsTool};
use kestrel_protocol::{Event, PROTOCOL_VERSION};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(name = "harness", version, about = "Headless Kestrel agent harness")]
struct Cli {
    /// Prompt to run.
    #[arg(short = 'p', long = "prompt")]
    prompt: String,
    /// Model id.
    #[arg(long)]
    model: Option<String>,
    /// Base URL for OpenAI-compatible API.
    #[arg(long)]
    base_url: Option<String>,
    /// Working directory.
    #[arg(long)]
    cwd: Option<PathBuf>,
    /// Directory containing SKILL.md skills.
    #[arg(long)]
    skills_dir: Option<PathBuf>,
    /// Skip loading ECC rules into system prompt (compact mode).
    #[arg(long)]
    compact: bool,
    /// Max agent turns.
    #[arg(long, default_value_t = 16)]
    max_turns: usize,
    /// Emit protocol events as JSON lines.
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let api_key = std::env::var("HARNESS_API_KEY").unwrap_or_default();
    let base_url = cli
        .base_url
        .or_else(|| std::env::var("HARNESS_BASE_URL").ok())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let model = cli
        .model
        .or_else(|| std::env::var("HARNESS_MODEL").ok())
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    let cwd = cli.cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let mut registry = kestrel_modules::SkillRegistry::new();
    // Load ECC skills
    let ecc = cwd.join("ecc").join("skills");
    if ecc.exists() { registry.load_dir(&ecc); }
    // Load impeccable skills
    let imp = cwd.join("impeccable").join(".cursor").join("skills");
    if imp.exists() { registry.load_dir(&imp); }
    // Load unslop skills
    let uns = cwd.join("unslop").join("skills");
    if uns.exists() { registry.load_dir(&uns); }
    // Optional override
    if let Some(d) = &cli.skills_dir {
        registry.load_dir(d);
    }
    // Load ECC rules into system prompt if present.
    let mut system_prompt = String::new();
    let rules_dir = cwd.join("ecc").join("rules");
    if !cli.compact && rules_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&rules_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    for f in std::fs::read_dir(&p).into_iter().flatten().flatten() {
                        let fp = f.path();
                        if fp.extension().map(|x| x == "md").unwrap_or(false) {
                            if let Ok(c) = std::fs::read_to_string(&fp) {
                                system_prompt.push_str(&c);
                                system_prompt.push_str("\n\n");
                            }
                        }
                    }
                }
            }
        }
    }
    let max_bytes = 32 * 1024;

    let provider = Arc::new(OpenAiProvider::new(base_url, api_key, model.clone()));
    let tools: Vec<Arc<dyn kestrel_core::Tool>> = vec![
        Arc::new(ReadTool::new(cwd.clone(), max_bytes)),
        Arc::new(EditTool::new(cwd.clone())),
        Arc::new(BashTool::new(cwd.clone(), max_bytes, std::time::Duration::from_secs(120))),
        Arc::new(SkillTool::new(registry.clone())),
        Arc::new(ListSkillsTool::new(registry.clone())),
    ];

    let (tx, mut rx) = mpsc::channel::<Event>(256);
    let cancel = CancellationToken::new();

    let config = AgentConfig {
        model,
        temperature: Some(0.2),
        max_tokens: None,
        max_turns: cli.max_turns,
        session_id: uuid::Uuid::new_v4().to_string(),
    };

    let handle = tokio::spawn(run_agent(
        provider,
        tools,
        if system_prompt.is_empty() { None } else { Some(system_prompt) },
        cli.prompt,
        config.clone(),
        tx,
        cancel.clone(),
    ));

    // Consume events.
    while let Some(ev) = rx.recv().await {
        if cli.json {
            println!("{}", serde_json::to_string(&ev).unwrap_or_default());
        } else {
            match ev {
                Event::MessageDelta { delta, .. } => {
                    use std::io::Write;
                    let _ = write!(std::io::stdout(), "{delta}");
                    let _ = std::io::stdout().flush();
                }
                Event::ToolCallStarted { call, .. } => {
                    eprintln!("\n[tool: {}]", call.name);
                }
                Event::ToolCallFinished { result, .. } => {
                    if result.is_error {
                        eprintln!("[tool error] {}", result.content);
                    }
                }
                Event::Error { message, .. } => {
                    eprintln!("error: {message}");
                }
                Event::SessionEnded { .. } => {
                    println!();
                }
                _ => {}
            }
        }
    }

    let _ = handle.await;
    let _ = PROTOCOL_VERSION;
    Ok(())
}
