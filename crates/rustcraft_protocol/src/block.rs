#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlockType {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
    Sand,
    Water,
    Wood,
    Leaves,
}

impl BlockType {
    pub fn is_solid(self) -> bool {
        !matches!(self, BlockType::Air)
    }

    pub fn is_transparent(self) -> bool {
        matches!(self, BlockType::Air | BlockType::Water)
    }

    /// Returns RGBA color as [r, g, b, a] in sRGB space.
    /// Use this in the protocol crate (no Bevy Color dependency).
    pub fn color_rgba(self) -> [f32; 4] {
        match self {
            BlockType::Air => [0.0, 0.0, 0.0, 0.0],
            BlockType::Grass => [0.33, 0.70, 0.24, 1.0],
            BlockType::Dirt => [0.55, 0.36, 0.20, 1.0],
            BlockType::Stone => [0.50, 0.50, 0.50, 1.0],
            BlockType::Sand => [0.87, 0.82, 0.57, 1.0],
            BlockType::Water => [0.20, 0.40, 0.80, 0.60],
            BlockType::Wood => [0.40, 0.26, 0.13, 1.0],
            BlockType::Leaves => [0.18, 0.55, 0.18, 1.0],
        }
    }
}
