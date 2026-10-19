//! The window-title module: the focused window's title, as waybar's
//! `hyprland/window` shows it (0.15.0's `src/modules/hyprland/window.cpp`).

use lyr_config::{Format, Window as Config};

/// What the module draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub text: String,
    pub tooltip: String,
}

/// What stands for the characters a cut title loses.
const ELLIPSIS: char = '\u{2026}';

/// `text` cut to `max` characters, the last of them an ellipsis, as Pango's
/// end ellipsis reads at waybar's `max-length`. Text that fits is kept.
fn cut(text: String, max: Option<u32>) -> String {
    let Some(max) = max.and_then(|max| usize::try_from(max).ok()) else {
        return text;
    };
    if text.chars().count() <= max {
        return text;
    }
    let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
    cut.push(ELLIPSIS);
    cut
}

/// The view of the window of `class` titled `title` under `config`. `None`
/// when it shows nothing, as with no window focused, so the module is left
/// out.
#[must_use]
pub fn view(class: &str, title: &str, config: &Config) -> Option<View> {
    let fill = |format: &Format| {
        format.render(|name| match name {
            "title" => Some(title),
            "class" => Some(class),
            _ => None,
        })
    };
    let text = cut(fill(&config.format), config.max_length);
    if text.is_empty() {
        return None;
    }
    let tooltip = config
        .tooltip_format
        .as_ref()
        .map_or_else(|| text.clone(), fill);
    Some(View { text, tooltip })
}

#[cfg(test)]
mod tests {
    use lyr_config::{Format, Platform, Window, from_sources};

    use super::view;

    fn defaults() -> Window {
        from_sources(None, Platform::Linux)
            .expect("defaults")
            .window
    }

    #[test]
    fn the_title_shows_and_is_its_own_tooltip() {
        let shown = view("kitty", "~/src: vim", &defaults()).expect("a view");
        assert_eq!(shown.text, "~/src: vim");
        assert_eq!(shown.tooltip, "~/src: vim");
        assert_eq!(view("", "", &defaults()), None, "no window");
    }

    #[test]
    fn a_long_title_is_cut_with_an_ellipsis() {
        let config = Window {
            max_length: Some(5),
            ..defaults()
        };
        assert_eq!(
            view("x", "abcdefgh", &config).expect("a view").text,
            "abcd\u{2026}"
        );
        assert_eq!(
            view("x", "abcde", &config).expect("a view").text,
            "abcde",
            "fits"
        );
    }

    #[test]
    fn a_tooltip_format_has_the_class_too() {
        let config = Window {
            format: Format::parse("{class}").expect("valid"),
            tooltip_format: Some(Format::parse("{class}: {title}").expect("valid")),
            ..defaults()
        };
        let shown = view("firefox", "lyr", &config).expect("a view");
        assert_eq!(shown.text, "firefox");
        assert_eq!(shown.tooltip, "firefox: lyr");
    }
}
