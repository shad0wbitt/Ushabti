//! App state and reducer.

use kestrel_protocol::{Command, Event, Message};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Chat,
    SessionPicker,
    Help,
}

#[derive(Debug, Clone)]
pub struct ToolBlock {
    pub name: String,
    pub args_summary: String,
    pub status: String,
    pub output: String,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub streaming: bool,
    pub tools: Vec<ToolBlock>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub screen: Screen,
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub cursor: usize,
    pub input_history: VecDeque<String>,
    pub history_idx: Option<usize>,
    pub model: String,
    pub session_id: String,
    pub tokens: u32,
    pub cost: f64,
    pub permission_mode: String,
    pub pending_approval: Option<PendingApproval>,
    pub scroll: usize,
    pub theme_dark: bool,
    pub small_terminal: bool,
    pub should_quit: bool,
    pub cancel_pending: bool,
}

#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub approval_id: String,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub reason: String,
}

impl AppState {
    pub fn new(model: String, session_id: String) -> Self {
        Self {
            screen: Screen::Chat,
            messages: Vec::new(),
            input: String::new(),
            cursor: 0,
            input_history: VecDeque::new(),
            history_idx: None,
            model,
            session_id,
            tokens: 0,
            cost: 0.0,
            permission_mode: "interactive".into(),
            pending_approval: None,
            scroll: 0,
            theme_dark: true,
            small_terminal: false,
            should_quit: false,
            cancel_pending: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Input {
    Key(char),
    Enter,
    ShiftEnter,
    Up,
    Down,
    CtrlC,
    Resize(u16, u16),
    CoreEvent(Event),
    Tick,
    Slash(String),
}

/// Pure reducer. Returns optional Command to send to core.
pub fn update(state: &mut AppState, input: Input) -> Option<Command> {
    match input {
        Input::Resize(w, h) => {
            state.small_terminal = w < 40 || h < 10;
            None
        }
        Input::CoreEvent(ev) => {
            match ev {
                Event::SessionStarted { session_id, .. } => {
                    state.session_id = session_id;
                    None
                }
                Event::MessageDelta { delta, .. } => {
                    if let Some(last) = state.messages.last_mut() {
                        if last.role == "assistant" && last.streaming {
                            last.content.push_str(&delta);
                            return None;
                        }
                    }
                    state.messages.push(ChatMessage {
                        role: "assistant".into(),
                        content: delta,
                        streaming: true,
                        tools: Vec::new(),
                    });
                    None
                }
                Event::MessageComplete { message, .. } => {
                    if let Some(last) = state.messages.last_mut() {
                        if last.role == "assistant" {
                            last.content = message.content.clone();
                            last.streaming = false;
                        }
                    }
                    None
                }
                Event::ToolCallStarted { call, .. } => {
                    let block = ToolBlock {
                        name: call.name.clone(),
                        args_summary: call.arguments.to_string(),
                        status: "running".into(),
                        output: String::new(),
                        collapsed: false,
                    };
                    if let Some(last) = state.messages.last_mut() {
                        last.tools.push(block);
                    } else {
                        state.messages.push(ChatMessage {
                            role: "assistant".into(),
                            content: String::new(),
                            streaming: false,
                            tools: vec![block],
                        });
                    }
                    None
                }
                Event::ToolCallFinished { result, .. } => {
                    for m in state.messages.iter_mut().rev() {
                        if let Some(tb) = m.tools.iter_mut().rev().find(|t| t.status == "running") {
                            tb.status = if result.is_error { "error".into() } else { "done".into() };
                            tb.output = result.content.clone();
                            break;
                        }
                    }
                    None
                }
                Event::ApprovalRequest { approval_id, tool, arguments, reason, .. } => {
                    state.pending_approval = Some(PendingApproval { approval_id, tool, arguments, reason });
                    None
                }
                Event::UsageUpdate { usage, .. } => {
                    state.tokens = usage.total_tokens;
                    // naive cost estimate
                    state.cost += (usage.prompt_tokens as f64 * 0.0000025) + (usage.completion_tokens as f64 * 0.00001);
                    None
                }
                Event::Error { message, .. } => {
                    state.messages.push(ChatMessage {
                        role: "system".into(),
                        content: format!("error: {message}"),
                        streaming: false,
                        tools: Vec::new(),
                    });
                    None
                }
                Event::SessionEnded { .. } => None,
                _ => None,
            }
        }
        Input::Key(c) => {
            if state.pending_approval.is_some() {
                return None; // modal blocks
            }
            if state.screen == Screen::Chat {
                state.input.insert(state.cursor, c);
                state.cursor += c.len_utf8();
            }
            None
        }
        Input::Enter => {
            if state.pending_approval.is_some() {
                return None;
            }
            if state.input.trim().is_empty() {
                return None;
            }
            let text = state.input.clone();
            state.input.clear();
            state.cursor = 0;
            state.input_history.push_front(text.clone());
            if state.input_history.len() > 100 {
                state.input_history.pop_back();
            }
            state.history_idx = None;
            // slash commands
            if text.starts_with('/') {
                let cmd = text.trim_start_matches('/').to_string();
                if cmd == "quit" { state.should_quit = true; return None; }
                if cmd == "help" { state.screen = Screen::Help; return None; }
                if cmd == "clear" { state.messages.clear(); return None; }
                if cmd == "sessions" { state.screen = Screen::SessionPicker; return None; }
                // other slash commands left as no-ops recorded
                state.messages.push(ChatMessage { role: "system".into(), content: format!("unknown slash: {cmd}"), streaming: false, tools: Vec::new() });
                return None;
            }
            state.messages.push(ChatMessage { role: "user".into(), content: text.clone(), streaming: false, tools: Vec::new() });
            Some(Command::SendMessage { session_id: state.session_id.clone(), content: text })
        }
        Input::ShiftEnter => {
            if state.pending_approval.is_some() { return None; }
            state.input.insert(state.cursor, '\n');
            state.cursor += 1;
            None
        }
        Input::Up => {
            if state.pending_approval.is_some() { return None; }
            if !state.input_history.is_empty() {
                let idx = state.history_idx.map(|i| i.saturating_add(1)).unwrap_or(0);
                if idx < state.input_history.len() {
                    state.history_idx = Some(idx);
                    state.input = state.input_history[idx].clone();
                    state.cursor = state.input.len();
                }
            }
            None
        }
        Input::Down => {
            if state.pending_approval.is_some() { return None; }
            match state.history_idx {
                Some(0) => { state.history_idx = None; state.input.clear(); state.cursor = 0; }
                Some(i) => { let ni = i - 1; state.history_idx = Some(ni); state.input = state.input_history[ni].clone(); state.cursor = state.input.len(); }
                None => {}
            }
            None
        }
        Input::CtrlC => {
            if state.cancel_pending {
                state.should_quit = true;
            } else {
                state.cancel_pending = true;
                // cancel current run
                return Some(Command::Cancel { session_id: state.session_id.clone() });
            }
            None
        }
        Input::Tick => {
            state.cancel_pending = false;
            None
        }
        Input::Slash(_) => None,
    }
}

/// Approval key handling helper.
pub fn handle_approval_key(state: &mut AppState, key: char) -> Option<Command> {
    let pa = state.pending_approval.clone()?;
    match key {
        'y' => {
            state.pending_approval = None;
            Some(Command::ApprovalResponse { approval_id: pa.approval_id, approved: true })
        }
        'a' => {
            state.pending_approval = None;
            Some(Command::ApprovalResponse { approval_id: pa.approval_id, approved: true })
        }
        'n' => {
            state.pending_approval = None;
            Some(Command::ApprovalResponse { approval_id: pa.approval_id, approved: false })
        }
        _ => None,
    }
}
