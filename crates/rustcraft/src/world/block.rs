use bevy::prelude::*;

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

    pub fn color(self) -> Color {
        match self {
            BlockType::Air => Color::NONE,
            BlockType::Grass => Color::srgb(0.33, 0.70, 0.24),
            BlockType::Dirt => Color::srgb(0.55, 0.36, 0.20),
            BlockType::Stone => Color::srgb(0.50, 0.50, 0.50),
            BlockType::Sand => Color::srgb(0.87, 0.82, 0.57),
            BlockType::Water => Color::srgba(0.20, 0.40, 0.80, 0.60),
            BlockType::Wood => Color::srgb(0.40, 0.26, 0.13),
            BlockType::Leaves => Color::srgb(0.18, 0.55, 0.18),
        }
    }
}
