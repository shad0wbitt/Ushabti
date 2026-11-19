//! Theme struct with dark and light palettes.

use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub muted: Color,
    pub error: Color,
    pub success: Color,
    pub border: Color,
    pub code_bg: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(18, 18, 24),
            fg: Color::Rgb(220, 220, 230),
            accent: Color::Rgb(120, 180, 255),
            muted: Color::Rgb(120, 120, 140),
            error: Color::Rgb(240, 100, 100),
            success: Color::Rgb(120, 220, 140),
            border: Color::Rgb(60, 60, 80),
            code_bg: Color::Rgb(30, 30, 40),
        }
    }
    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(245, 245, 250),
            fg: Color::Rgb(30, 30, 40),
            accent: Color::Rgb(40, 90, 180),
            muted: Color::Rgb(120, 120, 140),
            error: Color::Rgb(200, 60, 60),
            success: Color::Rgb(40, 140, 80),
            border: Color::Rgb(180, 180, 200),
            code_bg: Color::Rgb(230, 230, 240),
        }
    }
}
EOF