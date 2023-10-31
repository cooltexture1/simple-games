use crate::custom_game::{BuildGameError, CustomGame};
use rand::Rng;
use valence::prelude::*;

#[derive(PartialEq, Copy, Clone)]
enum Cell {
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

pub struct MineSweeperGame<const DIM: usize> {
    base_board: [[Cell; DIM]; DIM],
    current_board: [[CellState; DIM]; DIM],
    position: BlockPos,
    player: (Entity, UniqueId),
    is_build: bool,
    should_despawn: bool,
}

impl<const DIM: usize> MineSweeperGame<DIM> {
    pub fn new(pos: &Position, player: (Entity, UniqueId)) -> MineSweeperGame<DIM> {
        MineSweeperGame {
            base_board: Self::generate_board(40),
            current_board: [[CellState::Closed; DIM]; DIM],
            position: BlockPos::from(**pos),
            player,
            is_build: false,
            should_despawn: false,
        }
    }
    fn generate_board(bomb_amt: usize) -> [[Cell; DIM]; DIM] {
        let mut base = [[Cell::Empty; DIM]; DIM];
        let mut rng = rand::thread_rng();
        while base.flatten().iter().filter(|c| **c == Cell::Bomb).count() < bomb_amt {
            for cell in base.flatten_mut() {
                if rng.gen_ratio(1, 100) {
                    *cell = Cell::Bomb;
                }
            }
        }
        for x in 0..DIM {
            for y in 0..DIM {
                if base[y][x] == Cell::Bomb {
                    continue;
                } else {
                    if MineSweeperGame::<DIM>::count_bombs(&base, (x, y)) >= 1 {
                        base[y][x] =
                            Cell::Number(MineSweeperGame::<DIM>::count_bombs(&base, (x, y)));
                    }
                }
            }
        }
        return base;
    }

    fn count_bombs(base: &[[Cell; DIM]; DIM], (x, y): (usize, usize)) -> u8 {
        let mut bomb_count: u8 = 0;
        for scan_x in x as isize - 1..=x as isize + 1 {
            for scan_y in y as isize - 1..=y as isize + 1 {
                // if base[scan_y][scan_x] == Cell::Bomb {
                if base
                    .get(scan_y as usize)
                    .is_some_and(|y| y.get(scan_x as usize).is_some_and(|c| *c == Cell::Bomb))
                {
                    bomb_count += 1;
                }
            }
        }
        return bomb_count;
    }
}

impl<const DIM: usize> CustomGame for MineSweeperGame<DIM> {
    fn open_build_blocks(&mut self, layer: &mut ChunkLayer) -> Result<(), BuildGameError> {
        for x in 0..DIM {
            for y in 0..DIM {
                let pos = self.position.offset(x as i32, 0, y as i32);
                let block = get_num_color(self.base_board[y][x]);

                layer.set_block(pos, block);
            }
        }
        self.is_build = true;
        return Ok(());
    }
    fn build_blocks(&mut self, layer: &mut ChunkLayer) -> Result<(), BuildGameError> {
        for x in 0..DIM {
            for y in 0..DIM {
                let pos = self.position.offset(x as i32, 0, y as i32);
                let block = BlockState::MOSS_BLOCK;

                layer.set_block(pos, block);
            }
        }
        self.is_build = true;
        return Ok(());
    }
    fn tick(&mut self, layer: &mut ChunkLayer) {}
    fn click(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer) {
        for x in 0..DIM {
            for y in 0..DIM {
                let pos = self.position.offset(x as i32, 0, y as i32);
                if pos = click_pos {
                    match self.current_board[y][x] {
                        CellState::Closed => match self.base_board[y][x] {
                            Cell::Bomb => {
                                // play explosion sound
                                // reveal map
                                // set destroyed
                            }
                            Cell::Empty => {
                                // reveal click_pos
                                // play good sound
                                // check surrounding empty and simulate click
                            }
                            Cell::Number(n) => {
                                // play good sound
                                // reveal number
                            }
                        },
                        _ => (),
                    }
                }
            }
        }
    }
    // TODO add right click to flag
    fn reset(&self, layer: &mut ChunkLayer, pgsql: &mut crate::postgres_wrapper::PostgresWrapper) {}
    fn get_player(&self) -> (Entity, UniqueId) {
        self.player
    }
    fn should_despawn(&self) -> bool {
        self.should_despawn
    }
}

fn get_num_color(cell: Cell) -> BlockState {
    match cell {
        Cell::Empty => BlockState::STONE,
        Cell::Bomb => BlockState::TNT,
        Cell::Number(n) => match n {
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

// pub trait CustomGame {
//     fn build_blocks(&mut self, layer: &mut ChunkLayer) -> Result<(), BuildGameError>;

//     fn tick(&mut self, layer: &mut ChunkLayer);

//     fn click(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer);

//     fn reset(&self, layer: &mut ChunkLayer, pgsql: &mut PostgresWrapper);

//     fn should_despawn(&self) -> bool;

//     fn get_player(&self) -> (Entity, UniqueId);
// }
