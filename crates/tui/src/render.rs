//! Render function: pure function of AppState -> Frame.

use crate::app::{AppState, Screen};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn render(f: &mut Frame, state: &AppState) {
    let theme = if state.theme_dark { Theme::dark() } else { Theme::light() };
    if state.small_terminal {
        let p = Paragraph::new("Terminal too small").style(Style::default().fg(theme.error));
        f.render_widget(p, f.area());
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3), Constraint::Length(1)])
        .split(f.area());

    // messages
    let mut lines: Vec<Line> = Vec::new();
    for m in &state.messages {
        let color = match m.role.as_str() {
            "user" => theme.accent,
            "assistant" => theme.fg,
            "system" => theme.muted,
            _ => theme.fg,
        };
        lines.push(Line::from(Span::styled(
            format!("[{}] ", m.role),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        for l in m.content.lines() {
            lines.push(Line::from(Span::styled(l.to_string(), Style::default().fg(color))));
        }
        for tb in &m.tools {
            let status_color = if tb.status == "error" { theme.error } else { theme.success };
            lines.push(Line::from(vec![
                Span::styled(format!("  tool: {} ", tb.name), Style::default().fg(theme.accent)),
                Span::styled(format!("[{}]", tb.status), Style::default().fg(status_color)),
            ]));
            if !tb.collapsed {
                for l in tb.output.lines().take(8) {
                    lines.push(Line::from(Span::styled(format!("    {l}"), Style::default().fg(theme.muted))));
                }
            }
        }
    }
    let msg_widget = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Chat")).wrap(Wrap { trim: false });
    f.render_widget(msg_widget, chunks[0]);

    // input
    let input_widget = Paragraph::new(state.input.clone()).block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input_widget, chunks[1]);

    // status bar
    let status = format!("{} | {} | tokens:{} cost:{:.4} | {}", state.model, state.session_id, state.tokens, state.cost, state.permission_mode);
    let status_widget = Paragraph::new(status).style(Style::default().fg(theme.muted));
    f.render_widget(status_widget, chunks[2]);

    // approval modal overlay
    if let Some(pa) = &state.pending_approval {
        let area = centered_rect(60, 40, f.area());
        let text = format!("Approval required\nTool: {}\nReason: {}\nArgs: {}", pa.tool, pa.reason, pa.arguments);
        let modal = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Approve? y/a/n")).wrap(Wrap { trim: false });
        f.render_widget(modal, area);
    }

    // help overlay
    if state.screen == Screen::Help {
        let area = centered_rect(70, 60, f.area());
        let mut lines = Vec::new();
        lines.push(Line::from("Keybindings:"));
        for b in crate::keybindings::BINDINGS {
            lines.push(Line::from(format!("  {} - {}", b.keys, b.description)));
        }
        lines.push(Line::from(""));
        lines.push(Line::from("Slash: /help /clear /model /recipe /sessions /cost /quit"));
        let help = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
