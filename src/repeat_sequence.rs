use std::time::SystemTime;

use postgres::{Client, NoTls};
use rand::Rng;
use valence::{
    prelude::*,
    protocol::{sound::SoundCategory, Sound},
};

use crate::custom_game::CustomGame;

#[derive(PartialEq)]
pub enum GameState {
    Idle,
    Displaying,
    WaitForInput,
}

pub struct RepeatSequenceGame<const DIM: usize> {
    dir: Direction,
    wall_blocks: [[BlockPos; DIM]; DIM],
    button_blocks: [[BlockPos; DIM]; DIM],
    sequence: Vec<(BlockPos, BlockPos)>,
    player: (Entity, UniqueId),
    state: GameState,
    ticks: usize,
    input_progres: usize,
    should_despawn: bool,
    missed_clicks: usize,
}

impl<const DIM: usize> RepeatSequenceGame<DIM> {
    ///the position of the player starting the game, their yaw, and their entity
    pub fn new(pos: &Position, yaw: f32, player: (Entity, UniqueId)) -> RepeatSequenceGame<DIM> {
        let normalized_angle = yaw - (360.0 * yaw.div_euclid(360.0));
        let dir_num = (normalized_angle / 90.0).round();
        let dir = match dir_num as isize {
            0 => Direction::South,
            1 => Direction::West,
            2 => Direction::North,
            3 => Direction::East,
            4 => Direction::South,
            _ => unreachable!(),
        };
        // dir is now the direction the player is looking

        let bottom_left = Self::player_pos_to_bottom_left(pos, &dir);
        return RepeatSequenceGame::<DIM>::new_with_bottom_left(bottom_left, dir, player);
    }

    /// takes the bottom left block of the game, the direction of the game and the player
    pub fn new_with_bottom_left(
        bottom_left: BlockPos,
        dir: Direction,
        player: (Entity, UniqueId),
    ) -> RepeatSequenceGame<DIM> {
        let (wall_blocks, button_blocks) = Self::get_block_positions(&dir, &bottom_left);
        RepeatSequenceGame {
            dir,
            player,
            sequence: Vec::new(),
            wall_blocks,
            button_blocks,
            state: GameState::Idle,
            ticks: 0,
            input_progres: 0,
            should_despawn: false,
            missed_clicks: 0,
        }
    }

    /// turns the players position into the position of the bottom left block of the game
    fn player_pos_to_bottom_left(player_pos: &Position, dir: &Direction) -> BlockPos {
        let mut pos_block = BlockPos::new(
            player_pos.x as i32,
            player_pos.y as i32,
            player_pos.z as i32,
        );
        pos_block = pos_block.get_in_direction(*dir);
        pos_block = pos_block.get_in_direction(*dir);
        pos_block = pos_block.get_in_direction(*dir);
        pos_block = pos_block.get_in_direction(*dir);

        let offset: i32 = -(DIM as i32 / 2) - 1;
        match dir {
            Direction::North => pos_block.offset(offset, 0, 0),
            Direction::East => pos_block.offset(0, 0, offset),
            Direction::South => pos_block.offset(offset, 0, 0),
            Direction::West => pos_block.offset(0, 0, offset),
            _ => unreachable!(),
        }
    }

    // get the block_positions of all the blocks making up this instance
    fn get_block_positions(
        dir: &Direction,
        bottom_left: &BlockPos,
    ) -> ([[BlockPos; DIM]; DIM], [[BlockPos; DIM]; DIM]) {
        let opp_dir = opposite_dir(dir);
        let mut wall = [[BlockPos::new(0, 0, 0); DIM]; DIM];
        let mut buttons = [[BlockPos::new(0, 0, 0); DIM]; DIM];

        for y in 0..DIM {
            for xorz in 0..DIM {
                if *dir == Direction::North || *dir == Direction::South {
                    wall[xorz][y] = bottom_left.offset(xorz as i32, y as i32, 0);
                    buttons[xorz][y] = bottom_left
                        .offset(xorz as i32, y as i32, 0)
                        .get_in_direction(opp_dir);
                } else {
                    wall[xorz][y] = bottom_left.offset(0, y as i32, xorz as i32);
                    buttons[xorz][y] = bottom_left
                        .offset(0, y as i32, xorz as i32)
                        .get_in_direction(opp_dir);
                }
            }
        }
        return (wall, buttons);
    }

    /// generates a new step of the sequence
    fn generate_sequence(&mut self) {
        let mut rng = rand::thread_rng();
        let xcoord = rng.gen_range(0..DIM);
        let ycoord = rng.gen_range(0..DIM);

        if self
            .sequence
            .last()
            .unwrap_or(&(BlockPos::default(), BlockPos::default()))
            .0
            == self.button_blocks[xcoord][ycoord]
        {
            self.generate_sequence();
        } else {
            self.sequence.push((
                self.button_blocks[xcoord][ycoord],
                self.wall_blocks[xcoord][ycoord],
            ));
        }
    }
}

impl<const DIM: usize> CustomGame for RepeatSequenceGame<DIM> {
    fn reset(&self, layer: &mut ChunkLayer) {
        for block in self.wall_blocks.flatten() {
            layer.set_block(*block, BlockState::AIR);
        }
        for block in self.button_blocks.flatten() {
            layer.set_block(*block, BlockState::AIR);
        }
        let mut c = Client::connect("host=localhost user=postgres", NoTls).unwrap();
        let time = SystemTime::now();
        // TODO use the result
        c.execute(
            "INSERT INTO rsg_games (date, size, streak, player_uuid) VALUES ($1, $2, $3, $4)",
            &[
                &time,
                &(DIM as i32),
                &(self.sequence.len() as i32),
                &self.player.1.as_bytes().as_ref(),
            ],
        )
        .unwrap();
    }

    fn should_despawn(&self) -> bool {
        self.should_despawn
    }

    fn tick(&mut self, layer: &mut ChunkLayer) {
        self.ticks += 1;

        if self.state == GameState::Idle {
            if self.ticks > 20 {
                self.state = GameState::Displaying;
                self.ticks = 0;
                self.generate_sequence();
            }
        } else if self.state == GameState::Displaying {
            let display_step = self.ticks / 20;
            if self.ticks % 20 == 1 {
                //reset previous displayed block
                if display_step != 0 {
                    layer.set_block(
                        self.sequence.get(display_step - 1).unwrap().1,
                        BlockState::STONE,
                    );
                }
                //check if displaying is finished
                if display_step >= self.sequence.len() {
                    self.state = GameState::WaitForInput;
                    self.ticks = 0;
                    return;
                }
                //place the block to display
                layer.set_block(
                    self.sequence.get(display_step).unwrap().1,
                    BlockState::RED_CONCRETE,
                );
            }
        } else if self.state == GameState::WaitForInput {
            if self.ticks > (20 * 10) {
                self.should_despawn = true;
            }
        }
    }

    fn click(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer) {
        if player == self.player.0
            && self.state == GameState::WaitForInput
            && self.button_blocks.flatten().contains(&click_pos)
        {
            if self.sequence.get(self.input_progres).unwrap().0 == *click_pos {
                sound(layer, Sound::BlockNoteBlockBanjo, click_pos);
                self.input_progres += 1;
                if self.input_progres == self.sequence.len() {
                    self.state = GameState::Idle;
                    self.input_progres = 0;
                    sound(layer, Sound::BlockBeehiveEnter, click_pos);
                }
            } else {
                sound(layer, Sound::EntityCreeperDeath, click_pos);
                self.missed_clicks += 1;

                if self.missed_clicks >= 3 {
                    self.should_despawn = true;
                }
            }
        }
    }

    fn build_blocks(&self, layer: &mut ChunkLayer) {
        let opp_dir = opposite_dir(&self.dir);
        let wall_posistions = self.wall_blocks.flatten();
        let button_positions = self.button_blocks.flatten();

        for pos in wall_posistions {
            layer.set_block(*pos, BlockState::STONE);
        }
        let button_dir: PropValue;

        if self.dir == Direction::North || self.dir == Direction::South {
            button_dir = dir_to_prop_value(&self.dir);
        } else {
            button_dir = dir_to_prop_value(&opp_dir);
        }

        for pos in button_positions {
            layer.set_block(
                *pos,
                BlockState::OAK_BUTTON
                    .set(PropName::Face, PropValue::Wall)
                    .set(PropName::Facing, button_dir),
            );
        }
    }
}

fn dir_to_prop_value(dir: &Direction) -> PropValue {
    match dir {
        Direction::West => PropValue::West,
        Direction::South => PropValue::North,
        Direction::North => PropValue::South,
        Direction::Up => PropValue::Down,
        Direction::Down => PropValue::Up,
        Direction::East => PropValue::East,
    }
}

fn opposite_dir(dir: &Direction) -> Direction {
    match dir {
        Direction::South => Direction::North,
        Direction::North => Direction::South,
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

fn sound(layer: &mut ChunkLayer, sound: Sound, loc: &BlockPos) {
    layer.play_sound(
        sound,
        SoundCategory::Ambient,
        DVec3::new(loc.x.into(), loc.y.into(), loc.z.into()),
        20.0,
        1.0,
    );
}

// #[test]
// fn rsg_works() {
//     let rsg = RepeatSequenceGame::<5>::new(
//         &Position::new((79.0, -30.0, 78.0)),
//         28.0,
//         Entity::PLACEHOLDER,
//     );
// }
// fn rsg_fails_on_wrong_input() {
//
// }
