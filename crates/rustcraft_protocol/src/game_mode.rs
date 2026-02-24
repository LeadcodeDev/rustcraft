#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameMode {
    Creative,
    Survival,
}

impl Default for GameMode {
    fn default() -> Self {
        GameMode::Creative
    }
}
