use crate::custom_game::{BuildGameError, CustomGame};
use itertools::Itertools;
use rand::Rng;
use valence::{
    prelude::*,
    protocol::{sound::SoundCategory, Sound},
};

#[derive(PartialEq, Copy, Clone, Debug)]
enum CellContent {
    Empty,
    Bomb,
    Number(u8),
}

#[derive(PartialEq, Copy, Clone)]
enum CellState {
    Opened,
    Closed,
    Flagged,
}

// struct Cell {
//     content: CellContent,
//     state: CellState,
//     pos: BlockPos,
// }

const BOMB_AMT: usize = 40;

pub struct MineSweeperGame<const DIM: usize> {
    base_board: [[CellContent; DIM]; DIM],
    current_board: [[CellState; DIM]; DIM],
    position: BlockPos,
    player: (Entity, UniqueId),
    is_build: bool,
    should_despawn: bool,
    is_lost: bool,
    flag_lock: u8,
}

impl<const DIM: usize> MineSweeperGame<DIM> {
    pub fn new(pos: &Position, player: (Entity, UniqueId)) -> MineSweeperGame<DIM> {
        MineSweeperGame {
            base_board: Self::generate_board(BOMB_AMT),
            current_board: [[CellState::Closed; DIM]; DIM],
            position: BlockPos::from(**pos),
            player,
            is_build: false,
            should_despawn: false,
            is_lost: false,
            flag_lock: 0,
        }
    }
    fn generate_board(bomb_amt: usize) -> [[CellContent; DIM]; DIM] {
        let mut base = [[CellContent::Empty; DIM]; DIM];
        let mut rng = rand::thread_rng();
        while base
            .flatten()
            .iter()
            .filter(|c| **c == CellContent::Bomb)
            .count()
            < bomb_amt
        {
            for cell in base.flatten_mut() {
                if rng.gen_ratio(1, 100) {
                    *cell = CellContent::Bomb;
                }
            }
        }
        for (x, y) in (0..DIM).cartesian_product((0..DIM)) {
            if base[y][x] == CellContent::Bomb {
                continue;
            } else {
                if MineSweeperGame::<DIM>::count_bombs(&base, (x, y)) >= 1 {
                    base[y][x] =
                        CellContent::Number(MineSweeperGame::<DIM>::count_bombs(&base, (x, y)));
                }
            }
        }
        return base;
    }

    fn count_bombs(board: &[[CellContent; DIM]; DIM], (x, y): (usize, usize)) -> u8 {
        let mut bomb_count: u8 = 0;
        for scan_x in x as isize - 1..=x as isize + 1 {
            for scan_y in y as isize - 1..=y as isize + 1 {
                if board.get(scan_y as usize).is_some_and(|y| {
                    y.get(scan_x as usize)
                        .is_some_and(|c| *c == CellContent::Bomb)
                }) {
                    bomb_count += 1;
                }
            }
        }
        return bomb_count;
    }

    fn get_adjacent_cells(&self, (x, y): (usize, usize)) -> Vec<(usize, usize)> {
        let mut array: Vec<(usize, usize)> = vec![];
        for scan_x in x as isize - 1..=x as isize + 1 {
            for scan_y in y as isize - 1..=y as isize + 1 {
                if let Some(Some(cell)) = self
                    .base_board
                    .get(scan_y as usize)
                    .map(|y| y.get(scan_x as usize))
                {
                    array.push((scan_x as usize, scan_y as usize));
                }
            }
        }
        return array;
    }
}

impl<const DIM: usize> CustomGame for MineSweeperGame<DIM> {
    fn build_blocks(&mut self, layer: &mut ChunkLayer) -> Result<(), BuildGameError> {
        for (x, y) in (0..DIM).cartesian_product((0..DIM)) {
            let pos = self.position.offset(x as i32, 0, y as i32);
            let block = BlockState::MOSS_BLOCK;
            // let block = get_num_color(self.base_board[y][x]);
            layer.set_block(pos, block);
        }
        // for ((x, y), z) in (0..DIM)
        //     .cartesian_product((0..DIM))
        //     .cartesian_product((0..DIM))
        // {
        //     tracing::info!("{:?}", (x, y, z));
        // }

        self.is_build = true;
        return Ok(());
    }
    fn tick(&mut self, layer: &mut ChunkLayer) {
        if self.flag_lock > 0 {
            self.flag_lock -= 1;
        }
    }
    fn click_right(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer) {
        if self.flag_lock != 0 {
            return;
        }
        for (x, y) in (0..DIM).cartesian_product((0..DIM)) {
            let pos = self.position.offset(x as i32, 0, y as i32);
            if pos == *click_pos {
                self.flag_lock = 4;
                match self.current_board[y][x] {
                    CellState::Closed => {
                        layer.set_block(pos, BlockState::RED_WOOL);
                        self.current_board[y][x] = CellState::Flagged;
                    }
                    CellState::Flagged => {
                        layer.set_block(pos, BlockState::MOSS_BLOCK);
                        self.current_board[y][x] = CellState::Closed;
                    }
                    _ => (),
                }
            }
        }
    }
    fn click_left(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer) {
        if self.is_lost {
            self.should_despawn = true;
            return;
        }
        for (x, y) in (0..DIM).cartesian_product((0..DIM)) {
            if self.position.offset(x as i32, 0, y as i32) == *click_pos {
                match self.current_board[y][x] {
                    CellState::Closed => match self.base_board[y][x] {
                        CellContent::Bomb => {
                            if self
                                .current_board
                                .flatten()
                                .iter()
                                .all(|e| *e == CellState::Closed)
                            {
                                tracing::warn!("minesweeper: a bomb was the first clicked cell. Generating new Board.");
                                self.base_board = Self::generate_board(BOMB_AMT);
                                return;
                            }
                            sound(layer, Sound::EntityGenericExplode, click_pos);
                            for x in 0..DIM {
                                for y in 0..DIM {
                                    let pos = self.position.offset(x as i32, 0, y as i32);
                                    let block = get_num_color(self.base_board[y][x]);
                                    layer.set_block(pos, block);
                                }
                            }
                            self.is_lost = true;
                        }
                        CellContent::Empty => {
                            sound(layer, Sound::EntityFrogStep, click_pos);
                            self.current_board[y][x] = CellState::Opened;
                            let pos = self.position.offset(x as i32, 0, y as i32);
                            let block = get_num_color(self.base_board[y][x]);
                            layer.set_block(pos, block);
                            for adj in self.get_adjacent_cells((x, y)) {
                                if self.current_board[adj.1][adj.0] == CellState::Closed {
                                    let pos = self.position.offset(adj.0 as i32, 0, adj.1 as i32);
                                    self.click_left(&pos, player, layer);
                                }
                            }
                        }
                        CellContent::Number(n) => {
                            sound(layer, Sound::EntityFrogStep, click_pos);
                            self.current_board[y][x] = CellState::Opened;
                            let b =
                                layer.set_block(*click_pos, get_num_color(self.base_board[y][x]));
                            if !(b.clone().is_some_and(|b| b.state == BlockState::MOSS_BLOCK)) {
                                tracing::error!(
                                        "something went wrong clicking a minesweeper field, replaced block: {:?}",
                                        b.map(|b| b.state)
                                    );
                            }
                        }
                    },
                    _ => (),
                }
                //check if all are opened
            }
        }
    }
    fn reset(&self, layer: &mut ChunkLayer, pgsql: &mut crate::postgres_wrapper::PostgresWrapper) {
        for x in 0..DIM {
            for y in 0..DIM {
                let pos = self.position.offset(x as i32, 0, y as i32);
                layer.set_block(pos, BlockState::AIR);
            }
        }
        // TODO add database integration
    }
    fn get_player(&self) -> (Entity, UniqueId) {
        self.player
    }
    fn should_despawn(&self) -> bool {
        self.should_despawn
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

fn get_num_color(cell: CellContent) -> BlockState {
    match cell {
        CellContent::Empty => BlockState::STONE,
        CellContent::Bomb => BlockState::TNT,
        CellContent::Number(n) => match n {
            1 => BlockState::BLUE_GLAZED_TERRACOTTA,
            2 => BlockState::GREEN_GLAZED_TERRACOTTA,
            3 => BlockState::RED_GLAZED_TERRACOTTA,
            4 => BlockState::BLACK_GLAZED_TERRACOTTA,
            5 => BlockState::ORANGE_GLAZED_TERRACOTTA,
            6 => BlockState::LIGHT_BLUE_GLAZED_TERRACOTTA,
            7 => BlockState::PURPLE_GLAZED_TERRACOTTA,
            8 => BlockState::GRAY_GLAZED_TERRACOTTA,
            _ => {
                tracing::error!("unknown number of bombs: {}", n);
                unimplemented!();
            }
        },
    }
}
