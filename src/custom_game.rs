use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use crate::postgres_wrapper::PostgresWrapper;
use valence::{interact_block::InteractBlockEvent, prelude::*};

pub struct CustomGamePlugin;

impl Plugin for CustomGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_games,
                despawn_games,
                on_block_click,
                on_block_break,
                build_spawned_games,
            ),
        );
    }
}

fn build_spawned_games(
    mut layer: Query<&mut ChunkLayer>,
    mut new_games: Query<(Entity, &mut CustomGameContainer), Added<CustomGameContainer>>,
    mut clients: Query<&mut Client, &UniqueId>,
    mut commands: Commands,
) {
    for mut new_game in new_games.iter_mut() {
        if let Err(err) = new_game.1.build_blocks(layer.single_mut().as_mut()) {
            match clients.get_mut(new_game.1.get_player().0) {
                Ok(mut player) => {
                    player.send_chat_message(format!("Couldnt start game: {err}"));
                    commands.entity(new_game.0).despawn();
                }
                Err(err) => tracing::warn!("A games, player couldnt be found: {}", err),
            }
        }
    }
}

fn on_block_click(
    mut block_interacts: EventReader<InteractBlockEvent>,
    mut games: Query<&mut CustomGameContainer>,
    mut layer: Query<&mut ChunkLayer>,
) {
    for interaction in block_interacts.iter() {
        games.for_each_mut(|rsg| {
            rsg.into_inner().0.click_right(
                &interaction.position,
                interaction.client,
                layer.single_mut().into_inner(),
            )
        });
    }
}

fn on_block_break(
    mut block_interacts: EventReader<DiggingEvent>,
    mut games: Query<&mut CustomGameContainer>,
    mut layer: Query<&mut ChunkLayer>,
) {
    for interaction in block_interacts.iter() {
        games.for_each_mut(|rsg| {
            rsg.into_inner().0.click_left(
                &interaction.position,
                interaction.client,
                layer.single_mut().into_inner(),
            )
        });
    }
}

fn tick_games(mut games: Query<&mut CustomGameContainer>, mut layer: Query<&mut ChunkLayer>) {
    games.for_each_mut(|mut g| g.tick(layer.single_mut().into_inner()));
}

fn despawn_games(
    mut games: Query<(Entity, &mut CustomGameContainer)>,
    mut layer: Query<&mut ChunkLayer>,
    mut commands: Commands,
    mut database: Local<PostgresWrapper>,
) {
    games.for_each_mut(|g| {
        if g.1.should_despawn() {
            g.1.reset(layer.single_mut().into_inner(), database.deref_mut());
            commands.entity(g.0).despawn()
        }
    });
}

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
    fn build_blocks(&mut self, layer: &mut ChunkLayer) -> Result<(), BuildGameError>;

    fn tick(&mut self, layer: &mut ChunkLayer);

    fn click_right(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer);
    fn click_left(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer);

    fn reset(&self, layer: &mut ChunkLayer, pgsql: &mut PostgresWrapper);

    fn should_despawn(&self) -> bool;

    fn get_player(&self) -> (Entity, UniqueId);
}

#[derive(Debug)]
pub enum BuildGameError {
    BlocksInTheWay,
}

impl Display for BuildGameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildGameError::BlocksInTheWay => write!(f, "Error! There are Blocks in the Way"),
        }
    }
}
