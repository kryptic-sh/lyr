//! Reading one key from an INI-style file: GTK's `settings.ini`, and the
//! desktop entries of the Desktop Entry Specification.

/// The value of `key` in `[group]`, trimmed; the first one when the key
/// repeats. A key with a locale, such as `Icon[de]`, is another key.
#[must_use]
pub fn value<'a>(text: &'a str, group: &str, key: &str) -> Option<&'a str> {
    let mut in_group = false;
    for line in text.lines().map(str::trim) {
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            in_group = name == group;
        } else if in_group
            && let Some((name, value)) = line.split_once('=')
            && name.trim() == key
        {
            return Some(value.trim());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::value;

    #[test]
    fn a_key_is_read_from_its_own_group_only() {
        let text = "[One]\nIcon=a\n\n[Two]\nIcon[de] = b\nIcon = c\nIcon=d\n";
        assert_eq!(value(text, "Two", "Icon"), Some("c"), "trimmed, the first");
        assert_eq!(value(text, "One", "Icon"), Some("a"));
        assert_eq!(value(text, "Two", "Name"), None);
        assert_eq!(value(text, "Three", "Icon"), None);
    }
}
