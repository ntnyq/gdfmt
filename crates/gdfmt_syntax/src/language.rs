use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Explicit grammar profiles, not a claim of complete engine conformance.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum LanguageVersion {
    #[serde(rename = "4.3")]
    Godot4_3,
    #[serde(rename = "4.4")]
    Godot4_4,
    #[serde(rename = "4.5")]
    Godot4_5,
    #[serde(rename = "4.6")]
    Godot4_6,
    #[default]
    #[serde(rename = "4.7")]
    Godot4_7,
}

impl LanguageVersion {
    pub const ALL: [Self; 5] = [
        Self::Godot4_3,
        Self::Godot4_4,
        Self::Godot4_5,
        Self::Godot4_6,
        Self::Godot4_7,
    ];

    pub fn supports(self, feature: LanguageFeature) -> bool {
        self >= feature.since()
    }
}

impl fmt::Display for LanguageVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Godot4_3 => "4.3",
            Self::Godot4_4 => "4.4",
            Self::Godot4_5 => "4.5",
            Self::Godot4_6 => "4.6",
            Self::Godot4_7 => "4.7",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageFeature {
    TypedDictionary,
    ExportToolButton,
    WarningRegions,
    Abstract,
    VariadicFunction,
    ExportFilePath,
    MultilineExportHints,
}

impl LanguageFeature {
    pub const ALL: [Self; 7] = [
        Self::TypedDictionary,
        Self::ExportToolButton,
        Self::WarningRegions,
        Self::Abstract,
        Self::VariadicFunction,
        Self::ExportFilePath,
        Self::MultilineExportHints,
    ];

    pub const fn since(self) -> LanguageVersion {
        match self {
            Self::TypedDictionary | Self::ExportToolButton | Self::WarningRegions => {
                LanguageVersion::Godot4_4
            }
            Self::Abstract | Self::VariadicFunction | Self::ExportFilePath => {
                LanguageVersion::Godot4_5
            }
            Self::MultilineExportHints => LanguageVersion::Godot4_6,
        }
    }
}
