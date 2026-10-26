use super::icon_theme_setting;

#[test]
fn the_icon_theme_is_gtk_s_setting() {
    assert_eq!(
        icon_theme_setting(
            "[Settings]\ngtk-theme-name=Adwaita\ngtk-icon-theme-name = Papirus-Dark\n"
        )
        .as_deref(),
        Some("Papirus-Dark")
    );
    assert_eq!(
        icon_theme_setting("[Settings]\ngtk-icon-theme-name=\"Papirus\"\n").as_deref(),
        Some("Papirus"),
        "quotes are not part of the name"
    );
    assert_eq!(
        icon_theme_setting("[Other]\ngtk-icon-theme-name=Papirus\n"),
        None,
        "only the Settings group"
    );
    assert_eq!(
        icon_theme_setting("[Settings]\ngtk-icon-theme-name=\n"),
        None
    );
}
