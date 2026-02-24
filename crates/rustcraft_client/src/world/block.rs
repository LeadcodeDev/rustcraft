pub use rustcraft_protocol::block::BlockType;

use bevy::color::Color;

/// Extension trait to get a Bevy `Color` from a `BlockType`.
pub trait BlockColor {
    fn color(self) -> Color;
}

impl BlockColor for BlockType {
    fn color(self) -> Color {
        let [r, g, b, a] = self.color_rgba();
        Color::srgba(r, g, b, a)
    }
}
