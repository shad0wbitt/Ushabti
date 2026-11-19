//! Terminal UI crate for Kestrel.

pub mod app;
pub mod keybindings;
pub mod render;
pub mod theme;

pub use app::{AppState, update, Input, PendingApproval};
pub use keybindings::{BINDINGS, Action, from_key};
pub use render::render;
pub use theme::Theme;
EOF