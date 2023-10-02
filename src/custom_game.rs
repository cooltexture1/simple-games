use std::ops::{Deref, DerefMut};

use valence::prelude::*;

#[derive(Component)]
pub struct CustomGameContainer(pub Box<dyn CustomGame + Send + Sync>);

impl Deref for CustomGameContainer {
    type Target = Box<dyn CustomGame + Send + Sync>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CustomGameContainer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait CustomGame {
    fn build_blocks(&self, layer: &mut ChunkLayer);

    fn tick(&mut self, layer: &mut ChunkLayer);

    fn click(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer);

    fn reset(&self, layer: &mut ChunkLayer);

    fn should_despawn(&self) -> bool;
}
