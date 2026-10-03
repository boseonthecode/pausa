use ratatui::style::Color;

pub const CYAN: Color = Color::Cyan;
pub const BRIGHT_BLUE: Color = Color::Indexed(39);
pub const WHITE: Color = Color::White;
pub const AMBER: Color = Color::Yellow;
pub const DIM_WHITE: Color = Color::DarkGray;

/// Heatmap intensity levels (GitHub‑style green scale).
pub const HEATMAP_0: Color = Color::Reset;
pub const HEATMAP_1: Color = Color::Indexed(22);
pub const HEATMAP_2: Color = Color::Indexed(28);
pub const HEATMAP_3: Color = Color::Indexed(70);
pub const HEATMAP_4: Color = Color::Indexed(46);
pub const HEATMAP_TODAY: Color = Color::Indexed(226); // bright yellow
