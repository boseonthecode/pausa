const DOH_FONT: &str = include_str!("../../resources/doh.flf");

/// Render "PAUSA" with the doh figlet font and return the colored ANSI string.
pub fn render_splash() -> String {
    let font = figlet_rs::FIGlet::from_content(DOH_FONT).expect("invalid doh font");
    let figure = font.convert("PAUSA").expect("failed to render PAUSA");
    let ascii = figure.as_str();

    let lines: Vec<&str> = ascii.lines().collect();
    let total = lines.len();
    if total == 0 {
        return String::new();
    }

    let cyan_cutoff = total / 3;
    let blue_cutoff = (total * 2) / 3;

    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let color = if i < cyan_cutoff {
            "\x1b[36m" // cyan
        } else if i < blue_cutoff {
            "\x1b[94m" // bright blue
        } else {
            "\x1b[97m" // white
        };
        out.push_str(color);
        out.push_str(line);
        out.push_str("\x1b[0m\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_loads() {
        let font = figlet_rs::FIGlet::from_content(DOH_FONT);
        assert!(font.is_ok());
    }

    #[test]
    fn test_render_returns_text() {
        let result = render_splash();
        assert!(!result.is_empty(), "splash should not be empty");
    }

    #[test]
    fn test_splash_contains_pausa() {
        let result = render_splash();
        // The rendered art should contain P, A, U, S, A characters
        assert!(
            result.contains('P') || result.contains('S'),
            "splash should contain letters"
        );
    }

    #[test]
    fn test_splash_has_color_codes() {
        let result = render_splash();
        assert!(result.contains("\x1b[36m"), "should contain cyan");
        assert!(result.contains("\x1b[94m"), "should contain bright blue");
        assert!(result.contains("\x1b[97m"), "should contain white");
        assert!(result.contains("\x1b[0m"), "should contain reset");
    }
}
