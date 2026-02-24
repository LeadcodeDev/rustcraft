pub mod block;
pub mod chunk;
pub mod generation;

use bevy::prelude::*;
use chunk::ChunkMap;
use generation::generate_world;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMap>()
            .add_systems(Startup, generate_world);
    }
}
