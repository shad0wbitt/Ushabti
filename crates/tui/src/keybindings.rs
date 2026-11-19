//! Single source of truth for keybindings.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Cancel,
    Send,
    Newline,
    HistoryUp,
    HistoryDown,
    ScrollUp,
    ScrollDown,
    ToggleHelp,
    ToggleTheme,
    ApproveOnce,
    ApproveSession,
    Deny,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub keys: &'static str,
    pub action: Action,
    pub description: &'static str,
}

pub const BINDINGS: &[Binding] = &[
    Binding { keys: "Ctrl+C", action: Action::Cancel, description: "Cancel run / quit (twice)" },
    Binding { keys: "Enter", action: Action::Send, description: "Send message" },
    Binding { keys: "Shift+Enter", action: Action::Newline, description: "New line in input" },
    Binding { keys: "Up", action: Action::HistoryUp, description: "Previous input" },
    Binding { keys: "Down", action: Action::HistoryDown, description: "Next input" },
    Binding { keys: "PageUp", action: Action::ScrollUp, description: "Scroll up" },
    Binding { keys: "PageDown", action: Action::ScrollDown, description: "Scroll down" },
    Binding { keys: "?", action: Action::ToggleHelp, description: "Toggle help" },
    Binding { keys: "t", action: Action::ToggleTheme, description: "Toggle theme" },
    Binding { keys: "y", action: Action::ApproveOnce, description: "Approve once" },
    Binding { keys: "a", action: Action::ApproveSession, description: "Approve for session" },
    Binding { keys: "n", action: Action::Deny, description: "Deny" },
];

pub fn from_key(ev: KeyEvent) -> Option<Action> {
    match (ev.code, ev.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Cancel),
        (KeyCode::Enter, KeyModifiers::NONE) => Some(Action::Send),
        (KeyCode::Enter, KeyModifiers::SHIFT) => Some(Action::Newline),
        (KeyCode::Up, _) => Some(Action::HistoryUp),
        (KeyCode::Down, _) => Some(Action::HistoryDown),
        (KeyCode::PageUp, _) => Some(Action::ScrollUp),
        (KeyCode::PageDown, _) => Some(Action::ScrollDown),
        (KeyCode::Char('?'), _) => Some(Action::ToggleHelp),
        (KeyCode::Char('t'), KeyModifiers::NONE) => Some(Action::ToggleTheme),
        (KeyCode::Char('y'), _) => Some(Action::ApproveOnce),
        (KeyCode::Char('a'), _) => Some(Action::ApproveSession),
        (KeyCode::Char('n'), _) => Some(Action::Deny),
        _ => None,
    }
}
