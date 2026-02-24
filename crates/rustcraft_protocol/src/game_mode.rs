#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GameMode {
    Creative,
    Survival,
}

impl Default for GameMode {
    fn default() -> Self {
        GameMode::Creative
    }
}
