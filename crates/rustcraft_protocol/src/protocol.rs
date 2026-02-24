use bevy_math::{IVec3, Vec3};
use serde::{Deserialize, Serialize};

use crate::block::BlockType;
use crate::game_mode::GameMode;
use crate::inventory::ItemStack;

pub type SequenceNumber = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockAction {
    Break,
    Place,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Authentication request to join a server.
    Connect {
        auth_code: String,
        player_name: String,
    },
    /// Graceful disconnect.
    Disconnect,
    /// Player input for one tick/frame.
    InputCommand {
        sequence: SequenceNumber,
        dt: f32,
        yaw: f32,
        pitch: f32,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        jump: bool,
        sneak: bool,
    },
    /// Block interaction request (break or place).
    BlockInteraction {
        action: BlockAction,
        origin: Vec3,
        direction: Vec3,
    },
    /// Drop item from inventory to world.
    DropItem {
        slot: usize,
        count: u32,
        direction: Vec3,
    },
    /// Toggle game mode request.
    ToggleGameMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Connection accepted — assigns a player ID.
    ConnectAccepted { player_id: u64 },
    /// Connection rejected with reason.
    ConnectRejected { reason: String },
    /// Another player joined the world.
    PlayerJoined {
        player_id: u64,
        name: String,
        position: Vec3,
    },
    /// A player left the world.
    PlayerLeft { player_id: u64 },
    /// Broadcast of a player's position to other clients.
    PlayerPositionUpdate {
        player_id: u64,
        position: Vec3,
        yaw: f32,
        pitch: f32,
    },
    /// Authoritative player state after processing input.
    PlayerStateUpdate {
        last_processed_input: SequenceNumber,
        position: Vec3,
        velocity_y: f32,
        grounded: bool,
    },
    /// A block in the world changed.
    BlockChanged {
        position: IVec3,
        new_type: BlockType,
    },
    /// Full chunk data (for initial load or chunk streaming).
    ChunkData {
        pos: (i32, i32),
        blocks: Vec<BlockType>,
    },
    /// Tell the client to unload a chunk from memory.
    ChunkUnload { pos: (i32, i32) },
    /// Inventory state update.
    InventoryUpdate {
        slots: Vec<Option<ItemStack>>,
        active_slot: usize,
    },
    /// A dropped item was spawned in the world.
    DroppedItemSpawned {
        id: u64,
        stack: ItemStack,
        position: Vec3,
        velocity: Vec3,
    },
    /// A dropped item was collected/despawned.
    DroppedItemRemoved { id: u64 },
    /// Game mode changed.
    GameModeChanged { mode: GameMode },
}
