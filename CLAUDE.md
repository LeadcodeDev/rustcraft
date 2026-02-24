# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo build            # Build the project
cargo run              # Build and run the game
cargo check            # Type-check without building
cargo clippy           # Lint with Clippy
cargo test             # Run all tests
cargo test test_name   # Run a single test
```

Rust edition 2024. Requires Rust 1.91+.

## Architecture

Rustcraft is a Minecraft-like voxel sandbox built on **Bevy 0.15** ECS game engine with **noise** (Perlin) for terrain generation.

### Plugin-Based Module Structure

The app is composed of 8 Bevy plugins registered in `src/main.rs`:

- **EventsPlugin** (`events.rs`) — Custom events for inter-system communication: `BlockPlaced`, `BlockRemoved`, `PlayerMoved`, `GameModeChanged`, `InventoryPickedUp`, `InventoryDropped`, `ItemDroppedToWorld`
- **WorldPlugin** (`world/`) — Chunk-based voxel storage and procedural terrain generation
- **RenderPlugin** (`render/`) — Mesh building and chunk rendering with greedy face culling
- **PlayerPlugin** (`player/`) — First-person camera, movement (Creative flight / Survival gravity+jump), game state management
- **InventoryPlugin** (`inventory/`) — 36-slot inventory (9 hotbar + 27 main), item stacks (max 64)
- **InteractionPlugin** (`interaction/`) — DDA raycasting for block targeting, left-click break / right-click place
- **UiPlugin** (`ui/`) — Hotbar HUD, full inventory screen with drag-and-drop, pause menu, block 3D previews
- **DroppedItemPlugin** (`dropped_item/`) — Physics simulation, rotation animation, proximity-based pickup

### Key Data Flow

1. **World storage**: `ChunkMap` resource holds a `HashMap<ChunkPos, Chunk>`. Each `Chunk` stores a flat `Vec<BlockType>` (16x64x16). Cross-chunk neighbor marking handled automatically.
2. **Block changes**: Interaction systems modify `ChunkMap`, then fire `BlockPlaced`/`BlockRemoved` events. `RenderPlugin` listens for dirty chunks and remeshes them.
3. **Game state**: `GameState` resource (Playing/Paused/InInventory) gates input systems. `GameMode` (Creative/Survival) toggles physics.
4. **Inventory flow**: Breaking blocks fires `InventoryPickedUp`; placing consumes from active slot. Dropping fires `ItemDroppedToWorld` which spawns `DroppedItem` entities.

### World Constants

- Chunk size: 16x64x16, World: 8x8 chunks (128x128 blocks), Perlin noise seed 42
- Block types: Air, Grass, Dirt, Stone, Sand, Water, Wood, Leaves

### Conventions

- Each module exports a plugin struct implementing `Plugin` (e.g., `pub struct FooPlugin; impl Plugin for FooPlugin`)
- Global state uses Bevy `Resource`; per-entity data uses `Component`
- Systems communicate through Bevy `Event`s with `EventReader`/`EventWriter`
- System ordering uses `.after()` for explicit dependencies
