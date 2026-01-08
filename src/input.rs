#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    NextCategory,
    PrevCategory,
    ContextMenu,
    AddApp,
    Quit,
}
