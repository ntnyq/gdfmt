//! Private layout algebra. Groups choose a flat or broken layout using the
//! remaining line, including the documents following the group.
use crate::FormatOptions;
use std::borrow::Cow;

#[derive(Debug)]
pub(crate) enum Doc<'a> {
    Text(Cow<'a, str>),
    Concat(Vec<Doc<'a>>),
    Group(Box<Doc<'a>>),
    Indent(Box<Doc<'a>>),
    /// Visual continuation indentation does not enter the lexer suite stack.
    ContinuationIndent(Box<Doc<'a>>),
    /// A delimiter ending an inline lambda must use the enclosing suite column.
    StatementIndent(Box<Doc<'a>>),
    Line(&'static str),
    HardLine,
    /// Start a line unless already at its beginning.
    EnsureLine,
    LineSuffix(String),
}

impl<'a> Doc<'a> {
    pub(crate) fn text(text: impl Into<Cow<'a, str>>) -> Self {
        Self::Text(text.into())
    }
    pub(crate) fn concat(parts: impl IntoIterator<Item = Self>) -> Self {
        let mut parts = parts.into_iter().filter(|part| match part {
            Self::Text(text) => !text.is_empty(),
            Self::Concat(parts) => !parts.is_empty(),
            _ => true,
        });
        let Some(first) = parts.next() else {
            return Self::Concat(Vec::new());
        };
        let Some(second) = parts.next() else {
            return first;
        };
        Self::Concat([first, second].into_iter().chain(parts).collect())
    }
    pub(crate) fn group(self) -> Self {
        Self::Group(Box::new(self))
    }
    pub(crate) fn concat_vec(mut parts: Vec<Self>) -> Self {
        parts.retain(|part| match part {
            Self::Text(text) => !text.is_empty(),
            Self::Concat(parts) => !parts.is_empty(),
            _ => true,
        });
        match parts.len() {
            0 => Self::Concat(Vec::new()),
            1 => parts.pop().unwrap(),
            _ => Self::Concat(parts),
        }
    }
    pub(crate) fn indent(self) -> Self {
        Self::Indent(Box::new(self))
    }
    pub(crate) fn continuation_indent(self) -> Self {
        Self::ContinuationIndent(Box::new(self))
    }
    pub(crate) fn statement_indent(self) -> Self {
        Self::StatementIndent(Box::new(self))
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Flat,
    Break,
}
#[derive(Clone, Copy)]
struct Frame<'a> {
    level: usize,
    statement_level: usize,
    mode: Mode,
    doc: &'a Doc<'a>,
}

pub(crate) fn columns(text: &str, start: usize, tab_width: usize) -> usize {
    text.chars().fold(start, |column, character| {
        if character == '\t' {
            column + tab_width - column % tab_width
        } else {
            column + 1
        }
    })
}

fn fits(
    mut column: usize,
    width: usize,
    first: Frame<'_>,
    rest: &[Frame<'_>],
    tab_width: usize,
) -> bool {
    let mut pending = vec![first];
    let mut rest = rest.iter().rev();
    loop {
        if column > width {
            return false;
        }
        let frame = match pending.pop().or_else(|| rest.next().copied()) {
            Some(frame) => frame,
            None => return true,
        };
        match frame.doc {
            Doc::Text(text) => {
                if text.contains('\n') {
                    return matches!(frame.mode, Mode::Break);
                }
                column = columns(text, column, tab_width);
            }
            Doc::LineSuffix(text) => {
                if matches!(frame.mode, Mode::Flat) {
                    return false;
                }
                column = columns(text, column, tab_width);
            }
            Doc::Concat(parts) => {
                pending.extend(parts.iter().rev().map(|doc| Frame { doc, ..frame }))
            }
            Doc::Group(doc)
            | Doc::Indent(doc)
            | Doc::ContinuationIndent(doc)
            | Doc::StatementIndent(doc) => pending.push(Frame { doc, ..frame }),
            Doc::Line(flat) => match frame.mode {
                Mode::Flat => column += flat.len(),
                Mode::Break => return true,
            },
            Doc::HardLine | Doc::EnsureLine => return matches!(frame.mode, Mode::Break),
        }
    }
}

pub(crate) fn print(
    doc: &Doc<'_>,
    options: &FormatOptions,
    start_column: usize,
    base_indent: &str,
    newline: &str,
) -> String {
    let mut pending = vec![Frame {
        level: 0,
        statement_level: 0,
        mode: Mode::Break,
        doc,
    }];
    let mut output = String::new();
    let mut column = start_column;
    let mut suffixes = String::new();
    let mut line_start = false;
    while let Some(frame) = pending.pop() {
        match frame.doc {
            Doc::Text(text) => {
                if !text.is_empty() {
                    if text.as_ref() == " " && (line_start || !suffixes.is_empty()) {
                        continue;
                    }
                    if !suffixes.is_empty() {
                        output.push_str(&suffixes);
                        suffixes.clear();
                        output.push_str(newline);
                        line_start = true;
                    }
                    if line_start {
                        let indentation =
                            format!("{base_indent}{}", options.indent.repeat(frame.level));
                        output.push_str(&indentation);
                        column = columns(&indentation, 0, options.tab_width);
                        line_start = false;
                    }
                    output.push_str(text);
                    column = if let Some((_, last)) = text.rsplit_once('\n') {
                        columns(last, 0, options.tab_width)
                    } else {
                        columns(text, column, options.tab_width)
                    };
                }
            }
            Doc::LineSuffix(text) => {
                suffixes.push_str(text);
                column = columns(text, column, options.tab_width);
            }
            Doc::Concat(parts) => {
                pending.extend(parts.iter().rev().map(|doc| Frame { doc, ..frame }))
            }
            Doc::Indent(doc) => pending.push(Frame {
                level: frame.level + 1,
                statement_level: frame.statement_level + 1,
                doc,
                ..frame
            }),
            Doc::ContinuationIndent(doc) => pending.push(Frame {
                level: frame.level + 1,
                doc,
                ..frame
            }),
            Doc::StatementIndent(doc) => pending.push(Frame {
                level: frame.statement_level,
                doc,
                ..frame
            }),
            Doc::Group(doc) => {
                let flat = Frame {
                    mode: Mode::Flat,
                    doc,
                    ..frame
                };
                let mode = if matches!(frame.mode, Mode::Flat)
                    || fits(
                        column,
                        options.line_width,
                        flat,
                        &pending,
                        options.tab_width,
                    ) {
                    Mode::Flat
                } else {
                    Mode::Break
                };
                pending.push(Frame { mode, doc, ..frame });
            }
            Doc::Line(flat) if matches!(frame.mode, Mode::Flat) => {
                if !line_start && suffixes.is_empty() {
                    output.push_str(flat);
                    column += flat.len();
                }
            }
            Doc::EnsureLine if output.is_empty() || output == "\u{feff}" => {}
            Doc::EnsureLine | Doc::Line(_) if line_start && suffixes.is_empty() => {}
            Doc::Line(_) | Doc::HardLine | Doc::EnsureLine => {
                output.push_str(&suffixes);
                suffixes.clear();
                output.push_str(newline);
                let indentation = format!("{base_indent}{}", options.indent.repeat(frame.level));
                column = columns(&indentation, 0, options.tab_width);
                line_start = true;
            }
        }
    }
    output.push_str(&suffixes);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_concatenation_reuses_its_buffer_and_keeps_layout_controls() {
        let mut parts = Vec::with_capacity(16);
        parts.push(Doc::text("a"));
        parts.push(Doc::text(""));
        parts.push(Doc::Line(""));
        parts.push(Doc::text("b"));
        let buffer = parts.as_ptr();
        let doc = Doc::concat_vec(parts);
        assert!(matches!(&doc, Doc::Concat(parts) if parts.as_ptr() == buffer));
        assert_eq!(print(&doc.group(), &options(1), 0, "", "\n"), "a\nb");
    }
    #[test]
    fn source_text_is_borrowed_while_generated_text_can_be_owned() {
        let source = String::from("中文 literal");
        let borrowed = Doc::text(source.as_str());
        assert!(matches!(&borrowed, Doc::Text(text) if text.as_ptr() == source.as_ptr()));
        assert_eq!(print(&borrowed, &options(100), 0, "", "\n"), source);
        let generated = Doc::text(format!("generated: {source}"));
        assert_eq!(
            print(&generated, &options(100), 0, "", "\n"),
            "generated: 中文 literal"
        );
    }
    fn options(width: usize) -> FormatOptions {
        FormatOptions {
            line_width: width,
            indent: "  ".into(),
            ..Default::default()
        }
    }
    #[test]
    fn token_wrappers_elide_empty_concats_but_retain_empty_break_opportunities() {
        let token = Doc::concat([
            Doc::concat([]),
            Doc::concat([Doc::text("token"), Doc::concat([])]),
            Doc::text(""),
        ]);
        assert!(matches!(&token, Doc::Text(text) if text == "token"));
        assert_eq!(print(&token, &options(1), 0, "", "\n"), "token");
        let breakable = Doc::concat([
            Doc::text("a"),
            Doc::concat([Doc::text(""), Doc::Line("")]),
            Doc::text("b"),
        ])
        .group();
        assert_eq!(print(&breakable, &options(1), 0, "", "\n"), "a\nb");
        assert_eq!(print(&breakable, &options(2), 0, "", "\n"), "ab");
    }
    #[test]
    fn fitting_includes_following_text_and_nested_groups_choose_independently() {
        let list = || {
            Doc::concat([
                Doc::text("["),
                Doc::concat([
                    Doc::Line(""),
                    Doc::text("a,"),
                    Doc::Line(" "),
                    Doc::text("b"),
                ])
                .indent(),
                Doc::Line(""),
                Doc::text("]"),
            ])
            .group()
        };
        assert_eq!(print(&list(), &options(6), 0, "", "\n"), "[a, b]");
        assert_eq!(
            print(
                &Doc::concat([list(), Doc::text("!!")]),
                &options(6),
                0,
                "",
                "\n"
            ),
            "[\n  a,\n  b\n]!!"
        );
        assert_eq!(
            print(
                &Doc::concat([
                    Doc::text("["),
                    Doc::concat([Doc::Line(""), list()]).indent(),
                    Doc::Line(""),
                    Doc::text("]")
                ])
                .group(),
                &options(8),
                0,
                "",
                "\n"
            ),
            "[[a, b]]"
        );
    }
    #[test]
    fn hard_lines_suffixes_and_indentation_are_deterministic() {
        let doc = Doc::concat([
            Doc::text("a"),
            Doc::LineSuffix(" # tail".into()),
            Doc::HardLine,
            Doc::concat([Doc::text("b"), Doc::HardLine]).indent(),
        ])
        .group();
        assert_eq!(
            print(&doc, &options(100), 0, "\t", "\r\n"),
            "a # tail\r\n\t  b\r\n"
        );
        assert_eq!(
            print(&Doc::text("unbreakable"), &options(1), 0, "", "\n"),
            "unbreakable"
        );
        assert_eq!(columns("\t中😀", 0, 4), 6);
    }
}
