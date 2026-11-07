//! Game world data: terrain, map features and their gameplay properties.
//! Pure logic — nothing here knows about graphics.

pub mod chunk;
pub mod harvest;
pub mod island;
pub mod map;
pub mod noise;
pub mod river;
pub mod terrain;

pub use chunk::{CHUNK_CELLS, CHUNK_PX, ChunkId};
pub use map::{TILE_SIZE, WorldMap};
pub use terrain::{Feature, Terrain, TileProps};
