//! Skills loader for Kestrel, inspired by ECC's skill system.

use async_trait::async_trait;
use kestrel_core::Tool;
use kestrel_protocol::{ToolContext, ToolOutput};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    skills: BTreeMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_dir(&mut self, dir: &Path) -> usize {
        let mut count = 0;
        if !dir.exists() {
            return 0;
        }
        // Each skill: dir/<name>/SKILL.md or dir/<name>.md
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_file = path.join("SKILL.md");
                    if skill_file.exists() {
                        if let Some(skill) = parse_skill(&skill_file) {
                            self.skills.insert(skill.name.clone(), skill);
                            count += 1;
                        }
                    }
                } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Some(skill) = parse_skill(&path) {
                        self.skills.insert(skill.name.clone(), skill);
                        count += 1;
                    }
                }
            }
        }
        count
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }
}

fn parse_skill(path: &Path) -> Option<Skill> {
    let raw = fs::read_to_string(path).ok()?;
    let (name, description, body) = split_frontmatter(&raw, path);
    Some(Skill {
        name,
        description,
        content: body.to_string(),
        path: path.to_path_buf(),
    })
}

fn split_frontmatter<'a>(raw: &'a str, path: &Path) -> (String, String, &'a str) {
    let default_name = path
        .parent()
        .and_then(|p| p.file_name())
        .or_else(|| path.file_stem())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "skill".to_string());

    if !raw.starts_with("---") {
        return (default_name, String::new(), raw);
    }
    let rest = &raw[3..];
    if let Some(end) = rest.find("\n---") {
        let fm = &rest[..end];
        let body = &rest[end + 4..];
        let body = body.trim_start_matches('\n');
        let mut name = default_name.clone();
        let mut description = String::new();
        for line in fm.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim();
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if k == "name" && !v.is_empty() {
                    name = v.to_string();
                } else if k == "description" {
                    description = v.to_string();
                }
            }
        }
        (name, description, body)
    } else {
        (default_name, String::new(), raw)
    }
}

pub struct SkillTool {
    registry: SkillRegistry,
}

impl SkillTool {
    pub fn new(registry: SkillRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "skill"
    }
    fn description(&self) -> &str {
        "Load and return the full content of a named skill (SKILL.md). Use to apply specialized workflows."
    }
    fn schema(&self) -> Value {
        let names: Vec<String> = self.registry.list().iter().map(|s| s.name.clone()).collect();
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "enum": names,
                    "description": "Skill name to load"
                }
            },
            "required": ["name"]
        })
    }
    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, kestrel_core::ToolError> {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
        match self.registry.get(name) {
            Some(skill) => Ok(ToolOutput {
                content: format!("# {}\n\n{}", skill.name, skill.content),
                is_error: false,
            }),
            None => Ok(ToolOutput {
                content: format!("unknown skill: {name}"),
                is_error: true,
            }),
        }
    }
}

pub struct ListSkillsTool {
    registry: SkillRegistry,
}

impl ListSkillsTool {
    pub fn new(registry: SkillRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for ListSkillsTool {
    fn name(&self) -> &str {
        "list_skills"
    }
    fn description(&self) -> &str {
        "List available skills with descriptions."
    }
    fn schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<ToolOutput, kestrel_core::ToolError> {
        let mut out = String::new();
        for s in self.registry.list() {
            out.push_str(&format!("- {} — {}\n", s.name, s.description));
        }
        if out.is_empty() {
            out = "no skills loaded".to_string();
        }
        Ok(ToolOutput { content: out, is_error: false })
    }
}

pub fn load_registry(dir: &Path) -> SkillRegistry {
    let mut r = SkillRegistry::new();
    r.load_dir(dir);
    r
}
