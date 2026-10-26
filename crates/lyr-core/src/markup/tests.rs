use super::{Span, Style, escape, links, openable, parse, plain};

const BOLD: Style = Style {
    bold: true,
    italic: false,
    underline: false,
    link: None,
};

fn span(text: &str, style: Style) -> Span {
    Span {
        text: text.to_owned(),
        style,
    }
}

#[test]
fn plain_text_is_one_span() {
    assert_eq!(
        parse("Battery low"),
        [span("Battery low", Style::default())]
    );
    assert_eq!(parse(""), []);
}

#[test]
fn the_spec_s_tags_style_their_text() {
    assert_eq!(
        parse("a <b>b</b> <i>i</i> <u>u</u> <a href=\"https://x\">link</a>"),
        [
            span("a ", Style::default()),
            span("b", BOLD),
            span(" ", Style::default()),
            span(
                "i",
                Style {
                    italic: true,
                    ..Style::default()
                }
            ),
            span(" ", Style::default()),
            span(
                "u",
                Style {
                    underline: true,
                    ..Style::default()
                }
            ),
            span(" ", Style::default()),
            span(
                "link",
                Style {
                    link: Some(0),
                    ..Style::default()
                }
            ),
        ]
    );
}

#[test]
fn nested_tags_combine_and_a_stray_close_closes_nothing() {
    let bold_italic = Style {
        bold: true,
        italic: true,
        ..Style::default()
    };
    assert_eq!(
        parse("<b>x<i>y</i><b>z</b>w</b></b>v"),
        [
            span("x", BOLD),
            span("y", bold_italic),
            span("zw", BOLD),
            span("v", Style::default()),
        ]
    );
}

#[test]
fn entities_and_character_references_are_decoded() {
    assert_eq!(
        plain(&parse(
            "Tom &amp; Jerry &lt;3 &gt; &quot;&apos; &#65;&#x42;"
        )),
        "Tom & Jerry <3 > \"' AB"
    );
}

#[test]
fn what_is_not_markup_shows_as_it_is() {
    for text in [
        "a < b",
        "x<br/>y",
        "<b",
        "<script>alert(1)</script>",
        "<a>no href",
        "AT&T",
        "&nbsp;",
        "&#;",
        "&#+65;",
        "&#x-41;",
        "&#x+41;",
        "&#1114112;",
        "&amp",
    ] {
        assert_eq!(plain(&parse(text)), text, "{text}");
        assert!(
            parse(text)
                .iter()
                .all(|span| span.style == Style::default()),
            "{text}"
        );
    }
}

#[test]
fn a_closing_tag_of_the_spec_s_is_markup_even_with_nothing_open() {
    assert_eq!(plain(&parse("<a>no href</a>")), "<a>no href");
    assert_eq!(
        plain(&parse("<a href=xyx>t</a>")),
        "<a href=xyx>t",
        "an unquoted href is no link"
    );
}

#[test]
fn an_image_is_left_out_and_a_link_takes_either_quote() {
    assert_eq!(
        plain(&parse("see <img src=\"/a.png\" alt=\"a\"/>this")),
        "see this"
    );
    assert_eq!(
        parse("<a href='x'>y</a>"),
        [span(
            "y",
            Style {
                link: Some(0),
                ..Style::default()
            }
        )]
    );
}

#[test]
fn escaped_text_reads_back_as_itself() {
    for text in ["<b>Caf\u{e9} & bar</b>", "&amp; <a href=\"x\">", "plain"] {
        let spans = parse(&escape(text));
        assert_eq!(plain(&spans), text);
        assert!(spans.iter().all(|span| span.style == Style::default()));
    }
    assert_eq!(escape("a<b>&c"), "a&lt;b&gt;&amp;c");
}

#[test]
fn each_link_keeps_its_href_and_an_index_of_its_own() {
    let body = "<a href=\"https://a.example/?x=1&amp;y=2\">one</a> and <a href='http://b'>two</a>";
    assert_eq!(links(body), ["https://a.example/?x=1&y=2", "http://b"]);
    let spans = parse(body);
    let styled: Vec<(&str, Option<u32>)> = spans
        .iter()
        .map(|span| (span.text.as_str(), span.style.link))
        .collect();
    assert_eq!(
        styled,
        [("one", Some(0)), (" and ", None), ("two", Some(1))],
        "adjacent links stay apart"
    );
}

#[test]
fn only_an_http_or_https_link_opens() {
    for href in ["https://example.com", "HTTP://example.com/a b"] {
        assert!(openable(href), "{href}");
    }
    for href in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "mailto:a@b",
        "ftp://example.com",
        "httpsx://example.com",
        "https://",
        "example.com",
        "",
    ] {
        assert!(!openable(href), "{href}");
    }
}
