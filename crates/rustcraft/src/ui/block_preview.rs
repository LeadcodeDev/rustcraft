use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use crate::world::block::BlockType;

const PREVIEW_SIZE: u32 = 64;
const PREVIEW_RENDER_LAYER: usize = 10;

/// Stores a rendered preview image handle for each BlockType.
#[derive(Resource, Default)]
pub struct BlockPreviews {
    pub images: Vec<(BlockType, Handle<Image>)>,
}

impl BlockPreviews {
    pub fn get(&self, block: BlockType) -> Option<Handle<Image>> {
        self.images
            .iter()
            .find(|(b, _)| *b == block)
            .map(|(_, h)| h.clone())
    }
}

#[derive(Component)]
struct PreviewCube;

fn create_render_target(images: &mut Assets<Image>) -> Handle<Image> {
    let size = Extent3d {
        width: PREVIEW_SIZE,
        height: PREVIEW_SIZE,
        depth_or_array_layers: 1,
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("block_preview"),
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(size);

    images.add(image)
}

use bevy::render::view::visibility::RenderLayers;

pub fn setup_block_previews(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let preview_layer = RenderLayers::layer(PREVIEW_RENDER_LAYER);

    let block_types = [
        BlockType::Grass,
        BlockType::Dirt,
        BlockType::Stone,
        BlockType::Sand,
        BlockType::Wood,
        BlockType::Leaves,
        BlockType::Water,
    ];

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mut previews = BlockPreviews::default();

    // Light for all preview cubes
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(5.0, 10.0, 5.0),
        preview_layer.clone(),
    ));

    for (i, block) in block_types.iter().enumerate() {
        let image_handle = create_render_target(&mut images);

        // Offset each cube so they don't overlap
        let offset = Vec3::new(i as f32 * 5.0, -100.0, 0.0);

        // Camera for this block
        commands.spawn((
            Camera3d::default(),
            Camera {
                order: -(10 + i as isize),
                target: RenderTarget::Image(image_handle.clone().into()),
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            Transform::from_translation(offset + Vec3::new(1.2, 1.2, 1.2))
                .looking_at(offset, Vec3::Y),
            preview_layer.clone(),
        ));

        // Cube for this block
        commands.spawn((
            PreviewCube,
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: block.color(),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(offset),
            preview_layer.clone(),
        ));

        previews.images.push((*block, image_handle));
    }

    commands.insert_resource(previews);
}
