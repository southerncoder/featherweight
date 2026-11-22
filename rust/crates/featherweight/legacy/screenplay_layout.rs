pub type HorizontalAlignment = &'static str;

#[derive(Debug, Clone)]
pub struct LayoutAnchor {
    pub alignment: HorizontalAlignment,
    pub x: f64,
}

#[derive(Debug, Clone)]
pub struct DualDialogueLayout {
    pub left_character_cue_x: f64,
    pub left_dialogue_x: f64,
    pub right_character_cue_x: f64,
    pub right_dialogue_x: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ScreenplayLayout {
    pub action: Option<LayoutAnchor>,
    pub character_cue: Option<LayoutAnchor>,
    pub dialogue: Option<LayoutAnchor>,
    pub parenthetical: Option<LayoutAnchor>,
    pub transition: Option<LayoutAnchor>,
    pub dual_dialogue: Option<DualDialogueLayout>,
}