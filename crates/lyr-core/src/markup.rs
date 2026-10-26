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
    /// Inside `<a href="…">`: the link's index among the body's
    /// ([`links`]).
    pub link: Option<u32>,
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
    /// The innermost link opened, by its index.
    href: Option<u32>,
}

impl Depth {
    const fn style(self) -> Style {
        Style {
            bold: self.bold > 0,
            italic: self.italic > 0,
            underline: self.underline > 0,
            link: if self.link > 0 { self.href } else { None },
        }
    }
}

/// `body` as spans of text in their styles, joined where the style does not
/// change.
#[must_use]
pub fn parse(body: &str) -> Vec<Span> {
    scan(body).0
}

/// The `href` of each `<a>` in `body`, in order, its entities decoded: what
/// a span's [`Style::link`] indexes.
#[must_use]
pub fn links(body: &str) -> Vec<String> {
    scan(body).1
}

/// Whether a click may open `href`: an `http` or `https` URL, the scheme's
/// case aside (RFC 3986 section 3.1). Any other scheme, `file:` among them,
/// could run or reveal something on the user's machine on the word of
/// whichever program sent the notification.
#[must_use]
pub fn openable(href: &str) -> bool {
    href.split_once("://").is_some_and(|(scheme, rest)| {
        (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
            && !rest.is_empty()
    })
}

/// [`parse`] and [`links`] in one pass, so the two agree on which link is
/// which.
fn scan(body: &str) -> (Vec<Span>, Vec<String>) {
    let mut spans: Vec<Span> = Vec::new();
    let mut links = Vec::new();
    let mut depth = Depth::default();
    let mut rest = body;
    while let Some(next) = rest.chars().next() {
        let (text, used) = match next {
            '<' => match tag(rest) {
                Some((tag, used)) => {
                    apply(tag, &mut depth, &mut links);
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
    (spans, links)
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
enum Tag<'a> {
    Open(Name),
    Close(Name),
    /// `<a href="…">`, with its `href` as written.
    Link(&'a str),
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

fn apply(tag: Tag<'_>, depth: &mut Depth, links: &mut Vec<String>) {
    let (name, opens) = match tag {
        Tag::Open(name) => (name, true),
        Tag::Close(name) => (name, false),
        Tag::Link(href) => {
            depth.href = u32::try_from(links.len()).ok();
            links.push(decoded(href));
            (Name::Link, true)
        }
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
fn tag(text: &str) -> Option<(Tag<'_>, usize)> {
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
        _ if let Some(href) = inside
            .strip_prefix("a ")
            .and_then(|attributes| attribute(attributes, "href")) =>
        {
            Tag::Link(href)
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

/// `text` with its entities and character references decoded, as an
/// attribute's value is.
fn decoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let (piece, used) = entity(&rest[at..]).unwrap_or_else(|| (String::from('&'), 1));
        out.push_str(&piece);
        rest = &rest[at + used..];
    }
    out.push_str(rest);
    out
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
