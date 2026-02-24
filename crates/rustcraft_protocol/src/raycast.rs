use bevy_math::{IVec3, Vec3};

use crate::chunk::ChunkMap;

pub const MAX_REACH: f32 = 8.0;

pub struct RaycastHit {
    pub block_pos: IVec3,
    pub normal: IVec3,
}

pub fn dda_raycast(origin: Vec3, direction: Vec3, chunk_map: &ChunkMap) -> Option<RaycastHit> {
    let dir = direction.normalize();

    let mut pos = IVec3::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );

    let step = IVec3::new(
        if dir.x >= 0.0 { 1 } else { -1 },
        if dir.y >= 0.0 { 1 } else { -1 },
        if dir.z >= 0.0 { 1 } else { -1 },
    );

    let t_delta = Vec3::new(
        if dir.x != 0.0 {
            (1.0 / dir.x).abs()
        } else {
            f32::MAX
        },
        if dir.y != 0.0 {
            (1.0 / dir.y).abs()
        } else {
            f32::MAX
        },
        if dir.z != 0.0 {
            (1.0 / dir.z).abs()
        } else {
            f32::MAX
        },
    );

    let mut t_max = Vec3::new(
        if dir.x >= 0.0 {
            ((pos.x as f32 + 1.0) - origin.x) / dir.x
        } else if dir.x != 0.0 {
            (pos.x as f32 - origin.x) / dir.x
        } else {
            f32::MAX
        },
        if dir.y >= 0.0 {
            ((pos.y as f32 + 1.0) - origin.y) / dir.y
        } else if dir.y != 0.0 {
            (pos.y as f32 - origin.y) / dir.y
        } else {
            f32::MAX
        },
        if dir.z >= 0.0 {
            ((pos.z as f32 + 1.0) - origin.z) / dir.z
        } else if dir.z != 0.0 {
            (pos.z as f32 - origin.z) / dir.z
        } else {
            f32::MAX
        },
    );

    let mut normal = IVec3::ZERO;
    let max_steps = (MAX_REACH * 3.0) as i32;

    for _ in 0..max_steps {
        let block = chunk_map.get_block(pos.x, pos.y, pos.z);
        if block.is_solid() {
            return Some(RaycastHit {
                block_pos: pos,
                normal,
            });
        }

        if t_max.x < t_max.y && t_max.x < t_max.z {
            pos.x += step.x;
            t_max.x += t_delta.x;
            normal = IVec3::new(-step.x, 0, 0);
        } else if t_max.y < t_max.z {
            pos.y += step.y;
            t_max.y += t_delta.y;
            normal = IVec3::new(0, -step.y, 0);
        } else {
            pos.z += step.z;
            t_max.z += t_delta.z;
            normal = IVec3::new(0, 0, -step.z);
        }

        let dist_sq =
            (Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32) - origin).length_squared();
        if dist_sq > MAX_REACH * MAX_REACH {
            return None;
        }
    }

    None
}
