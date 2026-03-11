use std::path::PathBuf;

use crate::sim::world::World;

use super::PersistenceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveDescriptor {
    pub save_name: String,
    pub world_day: u64,
    pub country_count: u32,
    pub generation_seed: u64,
    pub updated_at: u64, // Unix timestamp
}

pub trait SaveRepository: Send + 'static {
    fn list_saves(&self) -> Result<Vec<SaveDescriptor>, PersistenceError>;
    fn save_world(&self, save_name: &str, world: &World) -> Result<(), PersistenceError>;
    fn load_world(&self, save_name: &str) -> Result<World, PersistenceError>;
    fn save_path(&self, save_name: &str) -> Result<PathBuf, PersistenceError>;
}
