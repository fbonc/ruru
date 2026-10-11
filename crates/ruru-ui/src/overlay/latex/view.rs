use std::borrow::Cow;

use iced::widget::{container, markdown, rich_text, row, svg, text};
use iced::{Center, Element, Fill, Length, Pixels};
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_parser::parse as parse_latex;
use ratex_svg::SvgOptions;
use ratex_types::{color::Color, math_style::MathStyle};

use super::super::answering::Input;
use super::super::style;

use super::parse::{Fragment, fragments};

pub(in crate::overlay) struct Viewer;

impl<'a> markdown::Viewer<'a, Input> for Viewer {
    fn on_link_click(url: markdown::Uri) -> Input {
        Input::LinkClicked(url)
    }

    fn paragraph(
        &self,
        settings: markdown::Settings,
        content: &markdown::Text,
    ) -> Element<'a, Input> {
        if let Some(source) = display_math(content, settings.style) {
            return math(&source, true, settings.text_size);
        }

        if has_math(content, settings.style) {
            inline_content(content, settings, settings.text_size)
        } else {
            markdown::paragraph(settings, content, Self::on_link_click)
        }
    }

    fn heading(
        &self,
        settings: markdown::Settings,
        level: &'a markdown::HeadingLevel,
        content: &'a markdown::Text,
        index: usize,
    ) -> Element<'a, Input> {
        let size = heading_size(settings, *level);

        if has_math(content, settings.style) {
            container(inline_content(content, settings, size))
                .padding(iced::padding::top(if index > 0 {
                    settings.text_size / 2.0
                } else {
                    Pixels::ZERO
                }))
                .into()
        } else {
            markdown::heading(settings, level, content, index, Self::on_link_click)
        }
    }
}

fn inline_content<'a>(
    content: &markdown::Text,
    settings: markdown::Settings,
    size: Pixels,
) -> Element<'a, Input> {
    let mut elements = Vec::new();

    for span in content.spans(settings.style).iter() {
        for fragment in fragments(&span.text) {
            match fragment {
                Fragment::Text(source) => {
                    for text_fragment in split_text(source) {
                        let mut styled = span.clone();
                        styled.text = Cow::Owned(text_fragment.to_owned());

                        elements.push(
                            rich_text(vec![styled])
                                .on_link_click(Input::LinkClicked)
                                .size(size)
                                .into(),
                        );
                    }
                }
                Fragment::Math { source, display } => {
                    elements.push(math(&source, display, size));
                }
            }
        }
    }

    row(elements)
        .width(Fill)
        .align_y(Center)
        .wrap()
        .vertical_spacing(0)
        .into()
}

fn math<'a>(source: &str, display: bool, size: Pixels) -> Element<'a, Input> {
    match render_math_svg(source, display, size) {
        Ok(bytes) => {
            let equation: Element<'a, Input> = svg::Svg::new(svg::Handle::from_memory(bytes))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into();

            if display {
                container(equation)
                    .center_x(Fill)
                    .padding(style::MATH_BLOCK_PADDING)
                    .into()
            } else {
                equation
            }
        }
        Err(_) => text(source.to_owned())
            .font(iced::Font::MONOSPACE)
            .size(size)
            .color(style::DANGER_COLOR)
            .into(),
    }
}

pub(super) fn render_math_svg(
    source: &str,
    display: bool,
    size: Pixels,
) -> Result<Vec<u8>, String> {
    let source = sanitize_math(source);
    let nodes = parse_latex(&source).map_err(|error| error.to_string())?;
    let color = style::TEXT_COLOR;
    let options = LayoutOptions {
        style: if display {
            MathStyle::Display
        } else {
            MathStyle::Text
        },
        color: Color::new(color.r, color.g, color.b, color.a),
        ..LayoutOptions::default()
    };
    let layout = layout(&nodes, &options);
    let display_list = to_display_list(&layout);
    let svg = ratex_svg::render_to_svg(
        &display_list,
        &SvgOptions {
            font_size: size.0 as f64,
            padding: 1.0,
            embed_glyphs: true,
            ..SvgOptions::default()
        },
    );

    Ok(svg.into_bytes())
}

pub(super) fn sanitize_math(source: &str) -> Cow<'_, str> {
    if !source.contains("&nbsp;")
        && !source.contains("&lt;")
        && !source.contains("&gt;")
        && !source.contains("&amp;")
        && !source.contains('\u{a0}')
    {
        return Cow::Borrowed(source);
    }

    Cow::Owned(
        source
            .replace("&nbsp;", r"\;")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", r"\&")
            .replace('\u{a0}', " "),
    )
}

fn display_math(content: &markdown::Text, style: markdown::Style) -> Option<String> {
    let source = content
        .spans(style)
        .iter()
        .map(|span| span.text.as_ref())
        .collect::<String>();
    let mut fragments = fragments(source.trim());

    match (fragments.next(), fragments.next()) {
        (
            Some(Fragment::Math {
                source,
                display: true,
            }),
            None,
        ) => Some(source),
        _ => None,
    }
}

fn has_math(content: &markdown::Text, style: markdown::Style) -> bool {
    content
        .spans(style)
        .iter()
        .any(|span| fragments(&span.text).any(|fragment| matches!(fragment, Fragment::Math { .. })))
}

fn split_text(source: &str) -> impl Iterator<Item = &str> {
    let mut start = 0;
    let mut fragments = Vec::new();

    for (index, character) in source.char_indices() {
        if character.is_whitespace() {
            let end = index + character.len_utf8();
            fragments.push(&source[start..end]);
            start = end;
        }
    }

    if start < source.len() {
        fragments.push(&source[start..]);
    }

    fragments
        .into_iter()
        .filter(|fragment| !fragment.is_empty())
}

fn heading_size(settings: markdown::Settings, level: markdown::HeadingLevel) -> Pixels {
    match level {
        markdown::HeadingLevel::H1 => settings.h1_size,
        markdown::HeadingLevel::H2 => settings.h2_size,
        markdown::HeadingLevel::H3 => settings.h3_size,
        markdown::HeadingLevel::H4 => settings.h4_size,
        markdown::HeadingLevel::H5 => settings.h5_size,
        markdown::HeadingLevel::H6 => settings.h6_size,
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::{Fragment, fragments, mark_math, parse};
    use super::*;

    #[test]
    fn multiline_display_math_with_delimiter_whitespace_is_recognized() {
        let source = r"$$ \frac{\partial}{\partial t}u(x,t)
\alpha \frac{\partial^2}{\partial x^2}u(x,t) $$

$$ \nabla\cdot\mathbf{E}=\frac{\rho}{\varepsilon_0}, \qquad
\nabla\times\mathbf{B}=\mu_0\mathbf{J}+\mu_0\varepsilon_0\frac{\partial\mathbf{E}}{\partial t} $$";
        let marked = mark_math(source);
        let parsed = fragments(&marked).collect::<Vec<_>>();
        let formulas = parsed
            .iter()
            .filter_map(|fragment| match fragment {
                Fragment::Math { source, display } => Some((source, display)),
                Fragment::Text(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(formulas.len(), 2);
        for (formula, display) in formulas {
            assert!(*display);
            render_math_svg(formula, true, Pixels(16.0))
                .expect("multiline display formula should render");
        }
    }

    #[test]
    fn common_latex_renders_to_svg() {
        let formulas = [
            ("scripts", r"x^2 + y_1 = 10"),
            ("fraction and root", r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"),
            ("greek", r"\alpha + \beta = \Gamma"),
            ("sum", r"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}"),
            ("integral", r"\int_0^1 x^2 \, dx = \frac{1}{3}"),
            (
                "limit and functions",
                r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
            ),
            ("delimiters", r"\left(\frac{a}{b}\right)"),
            ("matrix", r"\begin{bmatrix}a & b \\ c & d\end{bmatrix}"),
            (
                "cases",
                r"f(x)=\begin{cases}x^2 & x\ge 0 \\ -x & x<0\end{cases}",
            ),
            ("text", r"P(\text{heads}) = 0.5"),
            ("accents", r"\hat{x}, \bar{x}, \vec{x}"),
            ("math alphabets", r"\mathbb{R}, \mathbf{x}, \mathcal{F}"),
            ("set notation", r"A=\{3x:x\in 2\mathbb Z\}"),
            ("divisibility", r"6\mid(x-12)"),
        ];

        for (name, formula) in formulas {
            let svg = render_math_svg(formula, true, Pixels(16.0))
                .unwrap_or_else(|error| panic!("{name} failed to render: {error}"));
            let svg = String::from_utf8(svg).expect("SVG is UTF-8");

            assert!(svg.starts_with("<svg"), "{name} has no SVG root");
            assert!(svg.contains("viewBox="), "{name} has no viewport");
            assert!(svg.contains("<path"), "{name} has no rendered glyphs");
            assert!(svg.ends_with("</svg>"), "{name} has no closing SVG tag");
        }
    }

    #[test]
    fn math_survives_common_markdown_contexts() {
        let content = parse(
            r"# Energy $E=mc^2$

**Result:** $x=\frac{-b}{2a}$

$$\sum_{i=1}^n i$$",
        );
        let style = style::answer_markdown();

        for item in content.items() {
            let text = match item {
                markdown::Item::Heading(_, text) | markdown::Item::Paragraph(text) => text,
                _ => continue,
            };
            let source = text
                .spans(style)
                .iter()
                .map(|span| span.text.as_ref())
                .collect::<String>();

            for fragment in fragments(&source) {
                if let Fragment::Math { source, display } = fragment {
                    render_math_svg(&source, display, Pixels(16.0))
                        .unwrap_or_else(|error| panic!("integrated math failed: {error}"));
                }
            }
        }
    }

    #[test]
    fn fraction_shorthand_and_stretchy_braces_render() {
        render_math_svg(
            r"\left\{\frac12, \frac32, \frac52, \frac72\right\}",
            true,
            Pixels(16.0),
        )
        .expect("common TeX shorthand should render");
    }

    #[test]
    fn html_spacing_artifacts_are_removed_from_math() {
        let source = r"\left\{\frac{1}{2^n-1}: n \in \mathbb{Z},&nbsp;n \ge 1\right\}";
        let sanitized = sanitize_math(source);

        assert!(!sanitized.contains("&nbsp;"));
        render_math_svg(source, true, Pixels(16.0))
            .expect("HTML spacing artifacts should not break otherwise valid math");
    }
}
