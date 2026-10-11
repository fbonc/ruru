use std::borrow::Cow;
use std::ops::Range;

use iced::widget::markdown;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

const MATH_START: char = '\u{e000}';
const MATH_END: char = '\u{e001}';

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Fragment<'a> {
    Text(&'a str),
    Math { source: String, display: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExplicitDelimiter {
    Parentheses,
    Brackets,
    Dollars,
}

impl ExplicitDelimiter {
    fn opening(self) -> &'static str {
        match self {
            Self::Parentheses => r"\(",
            Self::Brackets => r"\[",
            Self::Dollars => "$$",
        }
    }

    fn closing(self) -> &'static str {
        match self {
            Self::Parentheses => r"\)",
            Self::Brackets => r"\]",
            Self::Dollars => "$$",
        }
    }

    fn display(self) -> bool {
        !matches!(self, Self::Parentheses)
    }
}

pub(in crate::overlay) fn parse(source: &str) -> markdown::Content {
    markdown::Content::parse(&mark_math(source))
}

pub(super) fn mark_math(source: &str) -> String {
    let source = mark_explicit_math(source);
    let parser = Parser::new_ext(&source, Options::ENABLE_MATH).into_offset_iter();
    let mut marked = String::with_capacity(source.len());
    let mut cursor = 0;

    for (event, range) in parser {
        let (math, display) = match event {
            Event::InlineMath(math) => (math, false),
            Event::DisplayMath(math) => (math, true),
            _ => continue,
        };

        marked.push_str(&source[cursor..range.start]);
        push_marker(&mut marked, &math, display);
        cursor = range.end;
    }

    marked.push_str(&source[cursor..]);
    marked
}

fn mark_explicit_math(source: &str) -> Cow<'_, str> {
    let protected = code_ranges(source);
    let mut output = None;
    let mut copied = 0;
    let mut search = 0;

    while let Some((opening, delimiter)) = find_opening(source, search, &protected) {
        let content_start = opening + delimiter.opening().len();
        let Some(closing) = find_delimiter(source, content_start, delimiter.closing(), &protected)
        else {
            search = content_start;
            continue;
        };

        let marked = output.get_or_insert_with(|| String::with_capacity(source.len()));
        marked.push_str(&source[copied..opening]);
        push_marker(
            marked,
            source[content_start..closing].trim(),
            delimiter.display(),
        );

        copied = closing + delimiter.closing().len();
        search = copied;
    }

    match output {
        Some(mut marked) => {
            marked.push_str(&source[copied..]);
            Cow::Owned(marked)
        }
        None => Cow::Borrowed(source),
    }
}

fn code_ranges(source: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut block_start = None;

    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => block_start = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = block_start.take() {
                    ranges.push(start..range.end);
                }
            }
            Event::Code(_) if block_start.is_none() => ranges.push(range),
            _ => {}
        }
    }

    ranges
}

fn find_opening(
    source: &str,
    start: usize,
    protected: &[Range<usize>],
) -> Option<(usize, ExplicitDelimiter)> {
    [
        ExplicitDelimiter::Parentheses,
        ExplicitDelimiter::Brackets,
        ExplicitDelimiter::Dollars,
    ]
    .into_iter()
    .filter_map(|delimiter| {
        find_delimiter(source, start, delimiter.opening(), protected)
            .map(|index| (index, delimiter))
    })
    .min_by_key(|(index, _)| *index)
}

fn find_delimiter(
    source: &str,
    start: usize,
    delimiter: &str,
    protected: &[Range<usize>],
) -> Option<usize> {
    source[start..]
        .match_indices(delimiter)
        .map(|(relative, _)| start + relative)
        .find(|index| {
            !protected.iter().any(|range| range.contains(index))
                && source[..*index]
                    .bytes()
                    .rev()
                    .take_while(|byte| *byte == b'\\')
                    .count()
                    % 2
                    == 0
        })
}

pub(super) fn push_marker(output: &mut String, source: &str, display: bool) {
    output.push(MATH_START);
    output.push(if display { 'd' } else { 'i' });
    output.push(':');

    for byte in source.as_bytes() {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }

    output.push(MATH_END);
}

pub(super) fn fragments(source: &str) -> impl Iterator<Item = Fragment<'_>> {
    let mut fragments = Vec::new();
    let mut cursor = 0;
    let mut search = 0;

    while let Some(relative_start) = source[search..].find(MATH_START) {
        let start = search + relative_start;
        let payload_start = start + MATH_START.len_utf8();
        let Some(relative_end) = source[payload_start..].find(MATH_END) else {
            break;
        };
        let end = payload_start + relative_end;
        let Some((display, math)) = decode_marker(&source[payload_start..end]) else {
            search = payload_start;
            continue;
        };

        if cursor < start {
            fragments.push(Fragment::Text(&source[cursor..start]));
        }
        fragments.push(Fragment::Math {
            source: math,
            display,
        });
        cursor = end + MATH_END.len_utf8();
        search = cursor;
    }

    if cursor < source.len() {
        fragments.push(Fragment::Text(&source[cursor..]));
    }

    fragments.into_iter()
}

fn decode_marker(payload: &str) -> Option<(bool, String)> {
    let (kind, encoded) = payload.split_once(':')?;
    let display = match kind {
        "i" => false,
        "d" => true,
        _ => return None,
    };

    if encoded.len() % 2 != 0 {
        return None;
    }

    let bytes = encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;

    String::from_utf8(bytes)
        .ok()
        .map(|source| (display, source))
}

#[cfg(test)]
mod tests {
    use super::super::super::style;
    use super::*;
    use iced::font;

    #[test]
    fn pulldown_cmark_identifies_inline_and_display_math() {
        let marked = mark_math("Euler: $e^{i\\pi} + 1 = 0$.\n\n$$\\sum_i x_i$$");
        let parsed = fragments(&marked).collect::<Vec<_>>();

        assert_eq!(
            parsed,
            vec![
                Fragment::Text("Euler: "),
                Fragment::Math {
                    source: "e^{i\\pi} + 1 = 0".into(),
                    display: false,
                },
                Fragment::Text(".\n\n"),
                Fragment::Math {
                    source: "\\sum_i x_i".into(),
                    display: true,
                },
            ]
        );
    }

    #[test]
    fn standard_tex_delimiters_become_math() {
        let marked = mark_math(r"Inline \(x^2\). Display: \[\sum_i x_i\]");

        assert_eq!(
            fragments(&marked).collect::<Vec<_>>(),
            vec![
                Fragment::Text("Inline "),
                Fragment::Math {
                    source: "x^2".into(),
                    display: false,
                },
                Fragment::Text(". Display: "),
                Fragment::Math {
                    source: "\\sum_i x_i".into(),
                    display: true,
                },
            ]
        );
    }

    #[test]
    fn math_is_not_recognized_inside_code() {
        let source = "`$inline$ \\(alternate\\)`\n\n```text\n$$display$$\n\\[alternate\\]\n```";
        assert_eq!(mark_math(source), source);
    }

    #[test]
    fn unmatched_and_escaped_alternate_delimiters_remain_text() {
        for source in [r"unmatched \[x", r"escaped \\[x\\]"] {
            assert_eq!(mark_math(source), source);
        }
    }

    #[test]
    fn prices_remain_text() {
        let source = "It costs $5 or $10.";
        assert_eq!(mark_math(source), source);
    }

    #[test]
    fn markers_round_trip_arbitrary_formula_text() {
        let mut marker = String::new();
        push_marker(&mut marker, r"x_{\text{owl}} = \$5", true);

        assert_eq!(
            fragments(&marker).collect::<Vec<_>>(),
            vec![Fragment::Math {
                source: r"x_{\text{owl}} = \$5".into(),
                display: true,
            }]
        );
    }

    #[test]
    fn iced_markdown_preserves_math_markers() {
        let content = parse("Before $x^2$ after");
        let Some(markdown::Item::Paragraph(text)) = content.items().first() else {
            panic!("expected a paragraph");
        };
        let source = text
            .spans(markdown::Style::from(iced::Theme::Dark))
            .iter()
            .map(|span| span.text.as_ref())
            .collect::<String>();

        assert_eq!(
            fragments(&source).collect::<Vec<_>>(),
            vec![
                Fragment::Text("Before "),
                Fragment::Math {
                    source: "x^2".into(),
                    display: false,
                },
                Fragment::Text(" after"),
            ]
        );
    }

    #[test]
    fn invalid_markers_remain_literal_text() {
        let source = "before \u{e000}not-a-marker\u{e001} after";
        assert_eq!(
            fragments(source).collect::<Vec<_>>(),
            vec![Fragment::Text(source)]
        );
    }

    #[test]
    fn common_markdown_blocks_are_preserved() {
        let content = parse(
            "# Heading\n\nParagraph\n\n> Quote\n\n- bullet\n\n1. ordered\n\n- [x] task\n\n---\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n```rust\nlet x = 1;\n```\n\n![owl](https://example.com/owl.png)",
        );
        let items = content.items();

        assert!(matches!(items.first(), Some(markdown::Item::Heading(..))));
        assert!(
            items
                .iter()
                .any(|item| matches!(item, markdown::Item::Paragraph(..)))
        );
        assert!(
            items
                .iter()
                .any(|item| matches!(item, markdown::Item::Quote(..)))
        );
        assert!(
            items
                .iter()
                .any(|item| matches!(item, markdown::Item::List { start: None, .. }))
        );
        assert!(
            items
                .iter()
                .any(|item| matches!(item, markdown::Item::List { start: Some(1), .. }))
        );
        assert!(items.iter().any(|item| matches!(
            item,
            markdown::Item::List { bullets, .. }
                if bullets.iter().any(|bullet| matches!(
                    bullet,
                    markdown::Bullet::Task { done: true, .. }
                ))
        )));
        assert!(
            items
                .iter()
                .any(|item| matches!(item, markdown::Item::Rule))
        );
        assert!(
            items
                .iter()
                .any(|item| matches!(item, markdown::Item::Table { .. }))
        );
        assert!(items.iter().any(|item| matches!(
            item,
            markdown::Item::CodeBlock { language, code, .. }
                if language.as_deref() == Some("rust") && code.contains("let x = 1;")
        )));
        assert!(items.iter().any(|item| matches!(
            item,
            markdown::Item::Image { url, .. }
                if url == "https://example.com/owl.png"
        )));
    }

    #[test]
    fn common_inline_markdown_styles_are_preserved() {
        let content =
            parse("plain **bold** *italic* ~~strike~~ `code` [link](https://example.com)");
        let Some(markdown::Item::Paragraph(text)) = content.items().first() else {
            panic!("expected a paragraph");
        };
        let style = style::answer_markdown();
        let spans = text.spans(style);
        let find = |needle: &str| {
            spans
                .iter()
                .find(|span| span.text.as_ref() == needle)
                .unwrap_or_else(|| panic!("missing {needle:?} span"))
        };

        assert_eq!(
            find("bold").font.expect("bold font").weight,
            font::Weight::Bold
        );
        assert_eq!(
            find("italic").font.expect("italic font").style,
            font::Style::Italic
        );
        assert!(find("strike").strikethrough);
        assert!(find("code").highlight.is_some());
        assert_eq!(find("link").link.as_deref(), Some("https://example.com"));
    }
}
