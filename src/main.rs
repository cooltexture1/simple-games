#![feature(slice_flatten)]

mod custom_game;
mod items;
mod minesweeper;
mod postgres_wrapper;
mod repeat_sequence;

use items::*;
use minesweeper::MineSweeperGame;
use minesweeper::MineSweeperGame3d;
use postgres::NoTls;
use postgres_wrapper::PostgresWrapper;
use repeat_sequence::RepeatSequenceGame;

use valence::message::ChatMessageEvent;
use valence::world_border::WorldBorderBundle;
use valence::{
    interact_item::InteractItemEvent,
    inventory::HeldItem,
    log::{Level, LogPlugin},
    prelude::*,
};

use custom_game::{CustomGameContainer, CustomGamePlugin};

fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            address: "0.0.0.0:25567".parse().unwrap(),
            ..Default::default()
        })
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
                chat_handler,
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
        c.execute(
            "CREATE TABLE IF NOT EXISTS minesweeper_games (
        date TIMESTAMP,
        size INT,
        dim INT,
        comp_time INT,
        bomb_amt INT,
        player_uuid BYTEA
);",
            &[],
        )
        .unwrap();
    } else {
        tracing::warn!("Couldnt establish database connection");
    }

    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
    for z in -7..7 {
        for x in -7..7 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -125..125 {
        for x in -25..25 {
            if z < 100 && z > -100 {
                layer.chunk.set_block([x, 64, z], BlockState::GRASS_BLOCK);
            } else {
                layer
                    .chunk
                    .set_block([x, 64, z], BlockState::DEEPSLATE_BRICKS);
            }
        }
    }
    let mut wb = WorldBorderBundle::default();
    wb.lerp.current_diameter = 150.0;
    wb.lerp.target_diameter = 150.0;
    commands.spawn((layer, wb));
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
            &Username,
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
        username,
    ) in &mut clients
    {
        tracing::info!("{} logged on", username);
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

        if let Some((size, dim, comp_time)) = database.get_minesweeper_fastest(uuid) {
            let d = match dim {
                2 => "2D",
                3 => "3D",
                _ => unreachable!(),
            };
            client.send_chat_message(format!(
                "Your fastest minesweeper game took: {} seconds, it was a {dim}x{dim} {d} game",
                comp_time / 20
            ));
        }

        client.set_resource_pack(
            "https://bits-mampfer.eu/tubnet-tourneys/minesweeper_resources.zip",
            "0ff6f1c2f43e03733090d08a44cecadccf2c532a",
            true,
            Some(Text::from("Install the minesweeper textures?")),
        );
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
                    let msg = MineSweeperGame::<20>::new(pos, (interaction.client, *uuid));
                    commands.spawn(CustomGameContainer(Box::new(msg)));
                }
                // StartItemType::Minesweeper3D20x20 => {
                //     let msg = MineSweeperGame3d::<20>::new(pos, (interaction.client, *uuid));
                //     commands.spawn(CustomGameContainer(Box::new(msg)));
                // }
                StartItemType::Minesweeper3D10x10 => {
                    let msg = MineSweeperGame3d::<10>::new(pos, (interaction.client, *uuid));
                    commands.spawn(CustomGameContainer(Box::new(msg)));
                }
                _ => (),
            }
        }
    }
}

fn chat_handler(
    mut messages: EventReader<ChatMessageEvent>,
    mut players: Query<(&mut Client, &Username)>,
) {
    for message in messages.iter() {
        let sender = players.get(message.client).unwrap().1 .0.clone();
        let msg: &str = message.message.as_ref();
        for mut player in players.iter_mut() {
            player.0.send_chat_message(format!("<{}> {}", sender, msg))
        }
    }
}
