use crossterm::event::{self, Event, KeyCode};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

fn main() -> anyhow::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    loop {
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(ratatui::widgets::Paragraph::new("Kestrel TUI (press q to quit)"), area);
        })?;
        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(k) = event::read()? {
                if k.code == KeyCode::Char('q') { break; }
            }
        }
    }
    terminal.clear()?;
    Ok(())
}
