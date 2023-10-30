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
}

impl<const DIM: usize> MineSweeperGame<DIM> {
    pub fn new(pos: &Position, player: (Entity, UniqueId)) -> MineSweeperGame<DIM> {
        MineSweeperGame {
            base_board: Self::generate_board(),
            current_board: [[CellState::Opened; DIM]; DIM],
            position: BlockPos::from(**pos),
            player,
            is_build: false,
        }
    }
    fn generate_board() -> [[Cell; DIM]; DIM] {
        let mut base = [[Cell::Empty; DIM]; DIM];
        let mut rng = rand::thread_rng();
        while base.flatten().iter().filter(|c| **c == Cell::Bomb).count() < 40 {
            for cell in base.flatten_mut() {
                if rng.gen_ratio(1, 100) {
                    *cell = Cell::Bomb;
                }
            }
        }
        for x in 0..=DIM {
            for y in 0..=DIM {
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
        for scan_x in x - 1..=x + 1 {
            for scan_y in y - 1..=y + 1 {
                if base[scan_y][scan_x] == Cell::Bomb {
                    bomb_count += 1;
                }
            }
        }
        return bomb_count;
    }
}

impl<const DIM: usize> CustomGame for MineSweeperGame<DIM> {
    fn build_blocks(&mut self, layer: &mut ChunkLayer) -> Result<(), BuildGameError> {
        for x in 0..=DIM {
            for y in 0..=DIM {
                let pos = self.position.offset(x as i32, 0, y as i32);
                let block = get_num_color(self.base_board[y][x]);

                layer.set_block(pos, block);
            }
        }
        return Ok(());
    }
    fn tick(&mut self, layer: &mut ChunkLayer) {}
    fn click(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer) {}
    fn reset(&self, layer: &mut ChunkLayer, pgsql: &mut crate::postgres_wrapper::PostgresWrapper) {}
    fn get_player(&self) -> (Entity, UniqueId) {
        self.player
    }
    fn should_despawn(&self) -> bool {
        false
    }
}

fn get_num_color(cell: Cell) -> BlockState {
    match cell {
        Cell::Empty => BlockState::STONE,
        Cell::Bomb => BlockState::TNT,
        Cell::Number(n) => match n {
            1 => BlockState::BLUE_WOOL,
            2 => BlockState::GREEN_WOOL,
            3 => BlockState::RED_WOOL,
            4 => BlockState::BLACK_WOOL,
            5 => BlockState::ORANGE_WOOL,
            6 => BlockState::CYAN_WOOL,
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
