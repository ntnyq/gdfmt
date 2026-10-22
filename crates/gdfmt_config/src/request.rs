use crate::ResolvedConfig;
use gdfmt_parser::LexOptions;

/// Map resolved host configuration into the pure text formatter.
pub fn format_request(
    config: &ResolvedConfig,
    source_id: gdfmt_text::SourceId,
) -> gdfmt::FormatRequest {
    gdfmt::FormatRequest {
        parse: gdfmt::ParseOptions {
            language_version: config.language.godot_version,
            lex: LexOptions {
                source_id,
                ..Default::default()
            },
            ..Default::default()
        },
        format: gdfmt::FormatOptions {
            line_width: config.format.line_width as usize,
            indent: match config.format.indent_style {
                crate::IndentStyle::Tab => "\t".into(),
                crate::IndentStyle::Space => " ".repeat(config.format.indent_width as usize),
            },
            tab_width: config.format.indent_width as usize,
            line_ending: match config.format.line_ending {
                crate::LineEnding::Lf => gdfmt::LineEnding::Lf,
                crate::LineEnding::Crlf => gdfmt::LineEnding::Crlf,
                crate::LineEnding::Preserve => gdfmt::LineEnding::Preserve,
            },
            max_blank_lines: config.format.max_blank_lines as usize,
            final_newline: config.format.final_newline,
            array_layout: match config.format.array_layout {
                crate::CollectionLayout::Auto => gdfmt::CollectionLayout::Auto,
                crate::CollectionLayout::Multiline => gdfmt::CollectionLayout::Multiline,
                crate::CollectionLayout::Preserve => gdfmt::CollectionLayout::Preserve,
            },
            dictionary_layout: match config.format.dictionary_layout {
                crate::CollectionLayout::Auto => gdfmt::CollectionLayout::Auto,
                crate::CollectionLayout::Multiline => gdfmt::CollectionLayout::Multiline,
                crate::CollectionLayout::Preserve => gdfmt::CollectionLayout::Preserve,
            },
            enum_layout: match config.format.enum_layout {
                crate::CollectionLayout::Auto => gdfmt::CollectionLayout::Auto,
                crate::CollectionLayout::Multiline => gdfmt::CollectionLayout::Multiline,
                crate::CollectionLayout::Preserve => gdfmt::CollectionLayout::Preserve,
            },
            dictionary_spacing: match config.format.dictionary_spacing {
                crate::DictionarySpacing::Compact => gdfmt::DictionarySpacing::Compact,
                crate::DictionarySpacing::Spaced => gdfmt::DictionarySpacing::Spaced,
            },
            trailing_comma_layout: match config.format.trailing_comma_layout {
                crate::TrailingCommaLayout::Auto => gdfmt::TrailingCommaLayout::Auto,
                crate::TrailingCommaLayout::Multiline => gdfmt::TrailingCommaLayout::Multiline,
            },
            blank_lines_around_definitions: match config.format.blank_lines_around_definitions {
                crate::BlankLinesAroundDefinitions::Preserve(_) => None,
                crate::BlankLinesAroundDefinitions::Count(count) => Some(count),
            },
            lambda_body_style: match config.format.lambda_body_style {
                crate::LambdaBodyStyle::Auto => gdfmt::LambdaBodyStyle::Auto,
                crate::LambdaBodyStyle::Multiline => gdfmt::LambdaBodyStyle::Multiline,
            },
            inline_comment_spacing: config.format.inline_comment_spacing,
        },
    }
}
