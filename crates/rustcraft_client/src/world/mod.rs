pub mod block;
pub mod chunk;

use bevy::prelude::*;
use chunk::ChunkMap;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMap>();
    }
}
