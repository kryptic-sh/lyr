use std::path::Path;

use lyr_config::{Colour, Custom, Platform, ReturnType, from_sources};

use super::{Output, View, parse, view};

/// The `[custom.test]` table `body` makes, on a bar that shows it.
fn custom(body: &str) -> Custom {
    let text = format!("[layout]\nleft = [\"custom/test\"]\n[custom.test]\n{body}");
    let mut config =
        from_sources(Some((Path::new("config.toml"), &text)), Platform::Linux).expect("valid");
    config.custom.remove(0)
}

fn lines(out: &str) -> Output {
    parse(out, ReturnType::Lines)
        .expect("lines always parse")
        .expect("an output")
}

fn json(out: &str) -> Output {
    parse(out, ReturnType::Json)
        .expect("valid JSON")
        .expect("an output")
}

fn shown(output: &Output, body: &str) -> Option<View> {
    view(output, &custom(body))
}

#[test]
fn i3blocks_lines_are_text_tooltip_and_class() {
    let output = lines("12 updates\r\nPackages to update\nwarning\nignored\n");
    assert_eq!(output.text, "12 updates", "a CRLF line ending is not text");
    assert_eq!(output.tooltip, "Packages to update");
    assert_eq!(output.class, ["warning"]);
    assert_eq!(output.percentage, 0);
    let single = lines("hello\n");
    assert_eq!(
        single.tooltip, "hello",
        "the text stands in for the tooltip"
    );
    assert!(single.class.is_empty());
}

#[test]
fn nothing_printed_is_no_output() {
    for out in ["", "\n", "\r\n"] {
        assert_eq!(parse(out, ReturnType::Lines), Ok(None), "{out:?}");
        assert_eq!(parse(out, ReturnType::Json), Ok(None), "{out:?}");
    }
}

#[test]
fn a_json_line_carries_every_field() {
    let output = json(
        r#"{"text": "T", "alt": "playing", "tooltip": "a\rb", "class": ["x", "y"], "percentage": 41.6}"#,
    );
    assert_eq!(
        output,
        Output {
            text: "T".to_owned(),
            alt: "playing".to_owned(),
            tooltip: "a\rb".to_owned(),
            class: vec!["x".to_owned(), "y".to_owned()],
            percentage: 42,
        }
    );
    let sparse =
        json("{\"text\": 5, \"class\": \"one\", \"percentage\": -3}\n{\"text\": \"later\"}");
    assert_eq!(sparse.text, "5", "a number is shown as written");
    assert_eq!(sparse.class, ["one"]);
    assert_eq!(sparse.percentage, 0);
    assert_eq!(sparse.tooltip, "", "no tooltip given is none");
}

#[test]
fn a_json_line_that_is_no_object_is_an_error() {
    for out in ["hello", "[1]", "\"text\"", "{\"text\":"] {
        assert!(parse(out, ReturnType::Json).is_err(), "{out:?}");
    }
}

#[test]
fn the_format_fills_text_alt_percentage_and_icon() {
    let output = json(r#"{"text": "song", "alt": "paused", "percentage": 70}"#);
    let view = shown(
        &output,
        "exec = \"x\"\nformat = \"{icon} {text} ({alt}, {percentage}%)\"\n\
         icons = { playing = \"P\", default = [\"lo\", \"hi\"] }\n",
    )
    .expect("shown");
    assert_eq!(
        view.text, "hi song (paused, 70%)",
        "no `paused`: the default ramp"
    );
    let playing = Output {
        alt: "playing".to_owned(),
        ..output.clone()
    };
    let view = shown(
        &playing,
        "exec = \"x\"\nformat = \"{icon}\"\nicons = { playing = \"P\" }\n",
    )
    .expect("shown");
    assert_eq!(view.text, "P");
    for (icons, expected) in [("[\"a\", \"b\", \"c\"]", "c"), ("\"S\"", "S")] {
        let view = shown(
            &output,
            &format!("exec = \"x\"\nformat = \"{{icon}}\"\nicons = {icons}\n"),
        )
        .expect("shown");
        assert_eq!(view.text, expected, "{icons}");
    }
    let bare = shown(&output, "exec = \"x\"\nformat = \"[{}]\"\n").expect("shown");
    assert_eq!(bare.text, "[song]", "a bare placeholder is the text");
}

#[test]
fn markup_is_read_unless_escaped() {
    let output = lines("<b>bold</b> &amp; plain\n");
    let read = shown(&output, "exec = \"x\"\n").expect("shown");
    assert_eq!(read.text, "bold & plain");
    let escaped = shown(
        &output,
        "exec = \"x\"\nescape = true\nformat = \"<i>{text}</i>\"\n",
    )
    .expect("shown");
    assert_eq!(
        escaped.text, "<b>bold</b> &amp; plain",
        "the script's text as printed, the format's own markup read"
    );
}

#[test]
fn an_empty_text_hides_the_module_or_not() {
    let empty = json(r#"{"text": ""}"#);
    assert_eq!(shown(&empty, "exec = \"x\"\n"), None, "nothing to show");
    let fixed = "exec = \"x\"\nformat = \"F {text}\"\n";
    assert_eq!(shown(&empty, fixed).expect("shown").text, "F ");
    assert_eq!(
        shown(&empty, &format!("{fixed}hide_empty_text = true\n")),
        None
    );
}

#[test]
fn a_static_module_is_its_format() {
    let view = shown(&Output::default(), "format = \"static\"\n").expect("shown");
    assert_eq!(view.text, "static");
    assert_eq!(view.tooltip.as_deref(), Some("static"));
    assert_eq!(shown(&Output::default(), ""), None, "no format, no text");
}

#[test]
fn the_tooltip_is_the_script_s_the_format_s_or_none() {
    let output = json(r#"{"text": "t", "tooltip": "line 1\rline <i>2</i>"}"#);
    let script = shown(&output, "exec = \"x\"\n").expect("shown");
    assert_eq!(script.tooltip.as_deref(), Some("line 1\nline 2"));
    let formatted = shown(&output, "exec = \"x\"\ntooltip_format = \"{text}!\"\n").expect("shown");
    assert_eq!(formatted.tooltip.as_deref(), Some("t!"), "the format wins");
    let off = shown(&output, "exec = \"x\"\ntooltip = false\n").expect("shown");
    assert_eq!(off.tooltip, None);
    let none = shown(&json(r#"{"text": "t"}"#), "exec = \"x\"\n").expect("shown");
    assert_eq!(none.tooltip, None, "a JSON line with no tooltip has none");
    let same = shown(
        &lines("t\n"),
        "exec = \"x\"\nformat = \"[{text}]\"\nmax_length = 2\n",
    )
    .expect("shown");
    assert_eq!(same.text, "[\u{2026}");
    assert_eq!(same.tooltip.as_deref(), Some("[t]"), "the whole label");
}

#[test]
fn the_first_class_with_a_colour_colours_the_label() {
    let body = "exec = \"x\"\ncolours = { warning = \"#ffaa00\", critical = \"#ff0000\" }\n";
    let colour = |class: &[&str]| {
        let output = Output {
            text: "t".to_owned(),
            class: class.iter().map(|&class| class.to_owned()).collect(),
            ..Output::default()
        };
        shown(&output, body).expect("shown").colour
    };
    assert_eq!(colour(&[]), None);
    assert_eq!(colour(&["other"]), None);
    assert_eq!(
        colour(&["other", "critical", "warning"]),
        Some(Colour::parse("#ff0000").expect("a colour"))
    );
}

#[test]
fn lengths_cut_and_pad_the_label() {
    let output = lines("abcdef\n");
    let cut = shown(&output, "exec = \"x\"\nmax_length = 4\n").expect("shown");
    assert_eq!(cut.text, "abc\u{2026}");
    let text = |extra: &str| {
        shown(
            &lines("ab\n"),
            &format!("exec = \"x\"\nmin_length = 6\n{extra}"),
        )
        .expect("shown")
        .text
    };
    assert_eq!(text(""), "  ab  ");
    assert_eq!(text("align = 0.0\n"), "ab    ");
    assert_eq!(text("align = 1.0\n"), "    ab");
    assert_eq!(
        shown(&output, "exec = \"x\"\nmin_length = 3\n")
            .expect("shown")
            .text,
        "abcdef",
        "long enough already"
    );
}
