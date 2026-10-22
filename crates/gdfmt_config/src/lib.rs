//! Validated JSON/JSONC configuration, field-wise merging, and generated JSON Schema.

mod request;
pub mod text;
pub use gdfmt_syntax::LanguageVersion;
pub use request::format_request;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
pub const MIN_LINE_WIDTH: u16 = 40;
pub const MAX_LINE_WIDTH: u16 = 320;
pub const MIN_INDENT_WIDTH: u8 = 1;
pub const MAX_INDENT_WIDTH: u8 = 8;
pub const MAX_BLANK_LINES: u8 = 5;

// Missing fields use Default; explicit null must not silently mean "missing".
fn non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum IndentStyle {
    #[default]
    Tab,
    Space,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LineEnding {
    #[default]
    Lf,
    Crlf,
    Preserve,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CollectionLayout {
    #[default]
    Auto,
    Multiline,
    Preserve,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DictionarySpacing {
    #[default]
    Compact,
    Spaced,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrailingCommaLayout {
    #[default]
    Auto,
    Multiline,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LambdaBodyStyle {
    #[default]
    Auto,
    Multiline,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PreserveBlankLines {
    #[default]
    Preserve,
}

/// Either retain existing spacing or normalize definition boundaries to a count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BlankLinesAroundDefinitions {
    Preserve(PreserveBlankLines),
    Count(#[schemars(range(min = 0, max = MAX_BLANK_LINES))] u8),
}

impl Default for BlankLinesAroundDefinitions {
    fn default() -> Self {
        Self::Preserve(PreserveBlankLines::Preserve)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialFormatOptions {
    /// Target print width; long indivisible tokens may exceed this value. Default: 100.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "u16", range(min = MIN_LINE_WIDTH, max = MAX_LINE_WIDTH))]
    pub line_width: Option<u16>,
    /// Indentation characters. Default: tab.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "IndentStyle")]
    pub indent_style: Option<IndentStyle>,
    /// Visual tab width or number of spaces per indentation level. Default: 4.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "u8", range(min = MIN_INDENT_WIDTH, max = MAX_INDENT_WIDTH))]
    pub indent_width: Option<u8>,
    /// Physical line endings. Default: lf; preserve uses the first newline, or lf.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "LineEnding")]
    pub line_ending: Option<LineEnding>,
    /// Maximum retained blank lines. Default: 2.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "u8", range(min = 0, max = MAX_BLANK_LINES))]
    pub max_blank_lines: Option<u8>,
    /// End formatted files with one newline. Default: true.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "bool")]
    pub final_newline: Option<bool>,
    /// Array literal layout. Default: auto.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "CollectionLayout")]
    pub array_layout: Option<CollectionLayout>,
    /// Dictionary literal layout. Default: auto.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "CollectionLayout")]
    pub dictionary_layout: Option<CollectionLayout>,
    /// Enum member layout. Default: auto.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "CollectionLayout")]
    pub enum_layout: Option<CollectionLayout>,
    /// Spaces inside single-line nonempty dictionaries. Default: compact.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "DictionarySpacing")]
    pub dictionary_spacing: Option<DictionarySpacing>,
    /// Expand nonempty lists with an existing trailing comma. Default: auto.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "TrailingCommaLayout")]
    pub trailing_comma_layout: Option<TrailingCommaLayout>,
    /// Blank lines around functions and classes in declaration scopes. Default: preserve.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "BlankLinesAroundDefinitions")]
    pub blank_lines_around_definitions: Option<BlankLinesAroundDefinitions>,
    /// Lambda body layout. Default: auto.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "LambdaBodyStyle")]
    pub lambda_body_style: Option<LambdaBodyStyle>,
    /// Spaces before inline comments. Default: 1.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "u8", range(min = 1, max = 4))]
    pub inline_comment_spacing: Option<u8>,
}

impl PartialFormatOptions {
    pub fn merge(&mut self, other: &Self) {
        macro_rules! merge { ($($field:ident),+) => { $(if other.$field.is_some() { self.$field = other.$field; })+ }; }
        merge!(
            line_width,
            indent_style,
            indent_width,
            line_ending,
            max_blank_lines,
            final_newline,
            array_layout,
            dictionary_layout,
            enum_layout,
            dictionary_spacing,
            trailing_comma_layout,
            blank_lines_around_definitions,
            lambda_body_style,
            inline_comment_spacing
        );
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialLanguageOptions {
    /// Explicit grammar profile. Default: 4.7. Does not imply engine conformance.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "LanguageVersion")]
    pub godot_version: Option<LanguageVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageOptions {
    pub godot_version: LanguageVersion,
}

/// Ordered, path-scoped layout/language changes. Paths are relative to the declaring config file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigOverride {
    /// Include globs, matched against the full relative path; an empty list matches nothing.
    pub files: Vec<String>,
    /// Exclude globs using the same path rules.
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "PartialLanguageOptions")]
    pub language: Option<PartialLanguageOptions>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "PartialFormatOptions")]
    pub format: Option<PartialFormatOptions>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartialConfig {
    /// Local JSON/JSONC files, applied in order before this file. No package or URL resolution.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "Vec<String>")]
    pub extends: Option<Vec<String>>,
    /// Ordered overrides append across inheritance layers and run after all base fields.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "Vec<ConfigOverride>")]
    pub overrides: Option<Vec<ConfigOverride>>,
    /// Gitignore-style patterns, relative to this file. Replaces an inherited list.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "Vec<String>")]
    pub ignore: Option<Vec<String>>,
    /// Editor metadata only; gdfmt never fetches this URL.
    #[serde(
        rename = "$schema",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "String")]
    pub schema: Option<String>,
    /// Configuration format version. Currently 1.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "u32", range(min = 1, max = 1))]
    pub schema_version: Option<u32>,
    /// Stop parent directory configuration discovery at this file. Default: false.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "bool")]
    pub root: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "PartialLanguageOptions")]
    pub language: Option<PartialLanguageOptions>,
    /// Script layout, indentation, line endings, blank lines and final newline.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null"
    )]
    #[schemars(with = "PartialFormatOptions")]
    pub format: Option<PartialFormatOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatOptions {
    pub line_width: u16,
    pub indent_style: IndentStyle,
    pub indent_width: u8,
    pub line_ending: LineEnding,
    pub max_blank_lines: u8,
    pub final_newline: bool,
    pub array_layout: CollectionLayout,
    pub dictionary_layout: CollectionLayout,
    pub enum_layout: CollectionLayout,
    pub dictionary_spacing: DictionarySpacing,
    pub trailing_comma_layout: TrailingCommaLayout,
    pub blank_lines_around_definitions: BlankLinesAroundDefinitions,
    pub lambda_body_style: LambdaBodyStyle,
    pub inline_comment_spacing: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedConfig {
    pub schema_version: u32,
    pub language: LanguageOptions,
    pub format: FormatOptions,
}

#[derive(Debug, Error)]
#[error("{field}: {message}")]
pub struct ConfigError {
    pub field: String,
    pub message: String,
}

impl PartialConfig {
    pub fn merge(&mut self, other: &Self) {
        if other.extends.is_some() {
            self.extends.clone_from(&other.extends);
        }
        if other.ignore.is_some() {
            self.ignore.clone_from(&other.ignore);
        }
        if let Some(overrides) = &other.overrides {
            self.overrides
                .get_or_insert_default()
                .extend(overrides.iter().cloned());
        }
        if other.schema.is_some() {
            self.schema.clone_from(&other.schema);
        }
        if other.schema_version.is_some() {
            self.schema_version = other.schema_version;
        }
        if other.root.is_some() {
            self.root = other.root;
        }
        if let Some(language) = &other.language {
            let current = self.language.get_or_insert_default();
            if language.godot_version.is_some() {
                current.godot_version = language.godot_version;
            }
        }
        if let Some(format) = &other.format {
            self.format.get_or_insert_default().merge(format);
        }
    }

    pub fn resolve(&self) -> Result<ResolvedConfig, ConfigError> {
        let schema_version = self.schema_version.unwrap_or(1);
        if schema_version != 1 {
            return Err(invalid(
                "schemaVersion",
                "only schema version 1 is supported",
            ));
        }
        if let Some(overrides) = &self.overrides {
            for (index, entry) in overrides.iter().enumerate() {
                PartialConfig {
                    language: entry.language.clone(),
                    format: entry.format.clone(),
                    ..Default::default()
                }
                .resolve()
                .map_err(|error| ConfigError {
                    field: format!("overrides[{index}].{}", error.field),
                    message: error.message,
                })?;
            }
        }
        let format = self.format.clone().unwrap_or_default();
        let line_width = format.line_width.unwrap_or(100);
        let indent_width = format.indent_width.unwrap_or(4);
        let max_blank_lines = format.max_blank_lines.unwrap_or(2);
        if !(MIN_LINE_WIDTH..=MAX_LINE_WIDTH).contains(&line_width) {
            return Err(invalid("format.lineWidth", "must be between 40 and 320"));
        }
        if !(MIN_INDENT_WIDTH..=MAX_INDENT_WIDTH).contains(&indent_width) {
            return Err(invalid("format.indentWidth", "must be between 1 and 8"));
        }
        if max_blank_lines > MAX_BLANK_LINES {
            return Err(invalid("format.maxBlankLines", "must be between 0 and 5"));
        }
        if format
            .inline_comment_spacing
            .is_some_and(|value| !(1..=4).contains(&value))
        {
            return Err(invalid(
                "format.inlineCommentSpacing",
                "must be between 1 and 4",
            ));
        }
        if let Some(BlankLinesAroundDefinitions::Count(value)) =
            format.blank_lines_around_definitions
            && value > MAX_BLANK_LINES
        {
            return Err(invalid(
                "format.blankLinesAroundDefinitions",
                "must be preserve or between 0 and 5",
            ));
        }
        Ok(ResolvedConfig {
            schema_version,
            language: LanguageOptions {
                godot_version: self
                    .language
                    .as_ref()
                    .and_then(|language| language.godot_version)
                    .unwrap_or_default(),
            },
            format: FormatOptions {
                line_width,
                indent_width,
                max_blank_lines,
                indent_style: format.indent_style.unwrap_or_default(),
                line_ending: format.line_ending.unwrap_or_default(),
                final_newline: format.final_newline.unwrap_or(true),
                array_layout: format.array_layout.unwrap_or_default(),
                dictionary_layout: format.dictionary_layout.unwrap_or_default(),
                enum_layout: format.enum_layout.unwrap_or_default(),
                dictionary_spacing: format.dictionary_spacing.unwrap_or_default(),
                trailing_comma_layout: format.trailing_comma_layout.unwrap_or_default(),
                blank_lines_around_definitions: format
                    .blank_lines_around_definitions
                    .unwrap_or_default(),
                lambda_body_style: format.lambda_body_style.unwrap_or_default(),
                inline_comment_spacing: format.inline_comment_spacing.unwrap_or(1),
            },
        })
    }
}

fn invalid(field: &str, message: &str) -> ConfigError {
    ConfigError {
        field: field.into(),
        message: message.into(),
    }
}

/// Parse JSON or JSONC with line/block comments and trailing commas.
/// Field validation is identical for both formats, independent of the file extension.
pub fn parse_config(source: &str) -> Result<PartialConfig, ConfigError> {
    let mut deserializer = serde_json_lenient::Deserializer::from_str(source);
    deserializer.set_allow_comments(true);
    deserializer.set_ignore_trailing_commas(true);
    let config: PartialConfig =
        serde_path_to_error::deserialize(&mut deserializer).map_err(|error| ConfigError {
            field: error.path().to_string(),
            message: error.inner().to_string(),
        })?;
    deserializer
        .end()
        .map_err(|error| invalid("$", &error.to_string()))?;
    config.resolve()?;
    Ok(config)
}

pub fn schema_json() -> String {
    let schema = schema_for!(PartialConfig);
    // Serializing an in-memory schema cannot fail.
    format!(
        "{}\n",
        serde_json::to_string_pretty(&schema).expect("schema is serializable")
    )
}
