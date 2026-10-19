//! A notification body's markup (`docs/plan/09-notifications.md`): the
//! Desktop Notifications spec's `<b>`, `<i>`, `<u>` and `<a href="…">`, with
//! XML's predefined entities and character references, since the spec calls
//! its markup XML-based. `<img>` is left out. Anything else, a tag the spec
//! does not list or one cut short, is shown as the text it is, never
//! interpreted.

/// How a span of text is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// Inside `<a href="…">`: drawn as a link, though not opened.
    pub link: bool,
}

/// A run of text in one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

/// How deep the text is in each tag, so `<b><b>x</b>y</b>` keeps `y` bold.
#[derive(Clone, Copy, Default)]
struct Depth {
    bold: u32,
    italic: u32,
    underline: u32,
    link: u32,
}

impl Depth {
    const fn style(self) -> Style {
        Style {
            bold: self.bold > 0,
            italic: self.italic > 0,
            underline: self.underline > 0,
            link: self.link > 0,
        }
    }
}

/// `body` as spans of text in their styles, joined where the style does not
/// change.
#[must_use]
pub fn parse(body: &str) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut depth = Depth::default();
    let mut rest = body;
    while let Some(next) = rest.chars().next() {
        let (text, used) = match next {
            '<' => match tag(rest) {
                Some((tag, used)) => {
                    apply(tag, &mut depth);
                    rest = &rest[used..];
                    continue;
                }
                None => (String::from('<'), 1),
            },
            '&' => entity(rest).unwrap_or_else(|| (String::from('&'), 1)),
            _ => {
                let end = rest.find(['<', '&']).unwrap_or(rest.len());
                (rest[..end].to_owned(), end)
            }
        };
        rest = &rest[used..];
        let style = depth.style();
        match spans.last_mut() {
            Some(last) if last.style == style => last.text.push_str(&text),
            _ => spans.push(Span { text, style }),
        }
    }
    spans
}

/// The text of `spans` without its styles.
#[must_use]
pub fn plain(spans: &[Span]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

/// `text` written so markup shows it as it is: `&`, `<` and `>` as XML's
/// entities.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// A tag the spec lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    Open(Name),
    Close(Name),
    /// `<img …>`, left out.
    Image,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Name {
    Bold,
    Italic,
    Underline,
    Link,
}

fn apply(tag: Tag, depth: &mut Depth) {
    let (name, opens) = match tag {
        Tag::Open(name) => (name, true),
        Tag::Close(name) => (name, false),
        Tag::Image => return,
    };
    let count = match name {
        Name::Bold => &mut depth.bold,
        Name::Italic => &mut depth.italic,
        Name::Underline => &mut depth.underline,
        Name::Link => &mut depth.link,
    };
    // A stray closing tag closes nothing.
    *count = if opens {
        *count + 1
    } else {
        count.saturating_sub(1)
    };
}

/// The tag `text` starts with, and its length, when it is one the spec
/// lists.
fn tag(text: &str) -> Option<(Tag, usize)> {
    let end = text.find('>')?;
    let inside = &text[1..end];
    let used = end + 1;
    let tag = match inside {
        "b" => Tag::Open(Name::Bold),
        "/b" => Tag::Close(Name::Bold),
        "i" => Tag::Open(Name::Italic),
        "/i" => Tag::Close(Name::Italic),
        "u" => Tag::Open(Name::Underline),
        "/u" => Tag::Close(Name::Underline),
        "/a" => Tag::Close(Name::Link),
        _ if inside.starts_with("a ") && attribute(&inside[2..], "href").is_some() => {
            Tag::Open(Name::Link)
        }
        _ if inside.starts_with("img ") => Tag::Image,
        _ => return None,
    };
    Some((tag, used))
}

/// The value of the attribute `name` among `attributes`, quoted with `"` or
/// `'`.
fn attribute<'a>(attributes: &'a str, name: &str) -> Option<&'a str> {
    let at = attributes.find(name)?;
    let value = attributes[at + name.len()..]
        .trim_start()
        .strip_prefix('=')?;
    let value = value.trim_start();
    let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let value = &value[1..];
    value.find(quote).map(|end| &value[..end])
}

/// The entity or character reference `text` starts with, decoded, and its
/// length.
fn entity(text: &str) -> Option<(String, usize)> {
    let end = text.find(';')?;
    let name = &text[1..end];
    let decoded = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        _ => {
            let number = name.strip_prefix('#')?;
            // Digits only: Rust's parsing would also take a sign.
            let code = match number.strip_prefix('x') {
                Some(hex) if hex.bytes().all(|b| b.is_ascii_hexdigit()) => {
                    u32::from_str_radix(hex, 16).ok()?
                }
                None if number.bytes().all(|b| b.is_ascii_digit()) => number.parse().ok()?,
                _ => return None,
            };
            char::from_u32(code)?
        }
    };
    Some((String::from(decoded), end + 1))
}

#[cfg(test)]
mod tests;
