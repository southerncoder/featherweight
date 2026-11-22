//! Port of `src/core/layout/screenplay-layout.ts`.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutRole {
    Action,
    CharacterCue,
    Dialogue,
    Parenthetical,
    Transition,
    DualDialogue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalAlignment {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayoutAnchor {
    pub alignment: HorizontalAlignment,
    pub x: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DualDialogueColumn {
    pub character_cue: LayoutAnchor,
    pub dialogue: LayoutAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DualDialogueLayout {
    pub left: DualDialogueColumn,
    pub right: DualDialogueColumn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LayoutDiagnosticCode {
    InsufficientLayoutEvidence,
    ConflictingLayoutEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutDiagnostic {
    pub code: LayoutDiagnosticCode,
    pub role: LayoutRole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayLayout {
    pub action: Option<LayoutAnchor>,
    pub character_cue: Option<LayoutAnchor>,
    pub dialogue: Option<LayoutAnchor>,
    pub parenthetical: Option<LayoutAnchor>,
    pub transition: Option<LayoutAnchor>,
    pub dual_dialogue: Option<DualDialogueLayout>,
    pub diagnostics: Vec<LayoutDiagnostic>,
}
