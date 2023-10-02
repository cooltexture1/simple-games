#![feature(slice_flatten)]

mod repeat_sequence;
use repeat_sequence::RepeatSequenceGame;
mod custom_game;
use custom_game::CustomGame;

use valence::{
    interact_block::InteractBlockEvent,
    interact_item::InteractItemEvent,
    inventory::HeldItem,
    log::{Level, LogPlugin},
    prelude::*,
};

use custom_game::CustomGameContainer;

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
                on_block_click,
                tick_games,
                despawn_games,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
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

    // layer.chunk.set_block(
    //     (0, 65, 0),
    //     Block::new(
    //         BlockState::OAK_BUTTON
    //             .set(PropName::Facing, PropValue::Up)
    //             .set(PropName::Face, PropValue::Floor),
    //         None,
    //     ),
    // );

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
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, 65.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}

fn item_use_listener(
    mut item_interacts: EventReader<InteractItemEvent>,
    players: Query<(&Look, &Inventory, &HeldItem, &Position, &VisibleChunkLayer)>,
    mut layers: Query<&mut ChunkLayer>,
    mut commands: Commands,
) {
    for interaction in item_interacts.iter() {
        let (look, inv, held_item, pos, player_layer) = players.get(interaction.client).unwrap();
        let held_item = inv.slot(held_item.slot());
        let layer: &mut ChunkLayer = layers.get_mut(player_layer.0).unwrap().into_inner();
        if held_item.item == ItemKind::Stick {
            let rsg5 = RepeatSequenceGame::<5>::new(pos, look.yaw, interaction.client);
            rsg5.build_blocks(layer);
            commands.spawn(CustomGameContainer(Box::new(rsg5)));
        } else if held_item.item == ItemKind::Diamond {
            let rsg7 = RepeatSequenceGame::<3>::new(pos, look.yaw, interaction.client);
            rsg7.build_blocks(layer);
            commands.spawn(CustomGameContainer(Box::new(rsg7)));
        }
    }
}

fn tick_games(mut games: Query<&mut CustomGameContainer>, mut layer: Query<&mut ChunkLayer>) {
    games.for_each_mut(|mut g| g.tick(layer.single_mut().into_inner()));
}

fn despawn_games(
    mut games: Query<(Entity, &mut CustomGameContainer)>,
    mut layer: Query<&mut ChunkLayer>,
    mut commands: Commands,
) {
    games.for_each_mut(|g| {
        if g.1.should_despawn() {
            g.1.reset(layer.single_mut().into_inner());
            commands.entity(g.0).despawn()
        }
    });
}

fn on_block_click(
    mut block_interacts: EventReader<InteractBlockEvent>,
    mut games: Query<&mut CustomGameContainer>,
    mut layer: Query<&mut ChunkLayer>,
) {
    for interaction in block_interacts.iter() {
        games.for_each_mut(|rsg| {
            rsg.into_inner().0.click(
                &interaction.position,
                interaction.client,
                layer.single_mut().into_inner(),
            )
        });
    }
}
