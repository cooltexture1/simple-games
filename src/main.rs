#![feature(slice_flatten)]

mod custom_game;
mod items;
mod minesweeper;
mod postgres_wrapper;
mod repeat_sequence;

use items::*;
use minesweeper::MineSweeperGame;
use postgres::NoTls;
use postgres_wrapper::PostgresWrapper;
use repeat_sequence::RepeatSequenceGame;

use valence::{
    interact_item::InteractItemEvent,
    inventory::HeldItem,
    log::{Level, LogPlugin},
    prelude::*,
};

use custom_game::{CustomGameContainer, CustomGamePlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            level: Level::INFO,
            ..Default::default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                item_use_listener,
            ),
        )
        .add_plugins(CustomGamePlugin)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    if let Ok(mut c) = postgres::Client::connect("host=localhost user=postgres", NoTls) {
        c.execute(
            "CREATE TABLE IF NOT EXISTS rsg_games (
        date TIMESTAMP,
        size INT,
        streak INT,
        player_uuid BYTEA
);",
            &[],
        )
        .unwrap();
    } else {
        tracing::warn!("Couldnt establish database connection");
    }

    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer.chunk.set_block([x, 64, z], BlockState::GRASS_BLOCK);
        }
    }
    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
            &mut Client,
            &mut Inventory,
            &UniqueId,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
    database: Local<PostgresWrapper>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
        mut client,
        mut inv,
        uuid,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, 65.0, 0.0]);
        *game_mode = GameMode::Creative;

        for e in StartItemType::all_types() {
            let slot_num = inv.first_empty_slot_in(36..45).unwrap();
            inv.set_slot(slot_num, StartItemType::create_start_item(e));
        }

        if let Some(streak) = database.get_highest_streak(uuid) {
            client.send_chat_message(format!("Your Highest Streak: {}", streak));
        }
    }
}

fn item_use_listener(
    mut item_interacts: EventReader<InteractItemEvent>,
    players: Query<(&Look, &Inventory, &HeldItem, &Position, &UniqueId)>,
    mut commands: Commands,
) {
    for interaction in item_interacts.iter() {
        let (look, inv, held_item, pos, uuid) = players.get(interaction.client).unwrap();
        let held_item = inv.slot(held_item.slot());
        if let Some(item_type) = StartItemType::get_start_item_type(held_item) {
            match item_type {
                StartItemType::RSG5 => {
                    let rsg5 =
                        RepeatSequenceGame::<5>::new(pos, look.yaw, (interaction.client, *uuid));
                    commands.spawn(CustomGameContainer(Box::new(rsg5)));
                }
                StartItemType::RSG7 => {
                    let rsg7 =
                        RepeatSequenceGame::<7>::new(pos, look.yaw, (interaction.client, *uuid));
                    commands.spawn(CustomGameContainer(Box::new(rsg7)));
                }
                StartItemType::Minesweeper => {
                    let msg = MineSweeperGame::<12>::new(pos, (interaction.client, *uuid));
                    commands.spawn(CustomGameContainer(Box::new(msg)));
                }
            }
        }
    }
}
