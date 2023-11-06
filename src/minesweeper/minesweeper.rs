use super::common::*;
use crate::custom_game::{BuildGameError, CustomGame};
use itertools::Itertools;
use rand::Rng;
use valence::{
    prelude::*,
    protocol::{sound::SoundCategory, Sound},
};

const BOMB_AMT: usize = 40;

pub struct MineSweeperGame<const DIM: usize> {
    board: [[Cell; DIM]; DIM],
    player: (Entity, UniqueId),
    is_build: bool,
    should_despawn: bool,
    is_over: bool,
    is_won: bool,
    flag_lock: u8,
    comp_time: usize,
}

impl<const DIM: usize> MineSweeperGame<DIM> {
    pub fn new(pos: &Position, player: (Entity, UniqueId)) -> MineSweeperGame<DIM> {
        MineSweeperGame {
            board: Self::generate_board(BOMB_AMT, BlockPos::from(**pos)),
            player,
            is_build: false,
            should_despawn: false,
            is_over: false,
            is_won: false,
            flag_lock: 0,
            comp_time: 0,
        }
    }
    fn generate_board(bomb_amt: usize, pos: BlockPos) -> [[Cell; DIM]; DIM] {
        let mut base = [[Cell::default(); DIM]; DIM];
        let mut rng = rand::thread_rng();
        // place all bombs
        while base
            .flatten()
            .iter()
            .filter(|c| c.content == CellContent::Bomb)
            .count()
            < bomb_amt
        {
            for cell in base.flatten_mut() {
                if rng.gen_ratio(1, 100) {
                    cell.content = CellContent::Bomb;
                }
            }
        }
        // fill in numbers and positions
        for (x, y) in (0..DIM).cartesian_product(0..DIM) {
            base[y][x].pos = pos.offset(x as i32, 0, y as i32);
            if base[y][x].content == CellContent::Bomb {
                continue;
            } else {
                if MineSweeperGame::<DIM>::count_bombs(&base, (x, y)) >= 1 {
                    base[y][x].content =
                        CellContent::Number(MineSweeperGame::<DIM>::count_bombs(&base, (x, y)));
                }
            }
        }
        return base;
    }

    fn count_bombs(board: &[[Cell; DIM]; DIM], (x, y): (usize, usize)) -> u8 {
        let mut bomb_count: u8 = 0;
        for scan_x in x as isize - 1..=x as isize + 1 {
            for scan_y in y as isize - 1..=y as isize + 1 {
                if board.get(scan_y as usize).is_some_and(|y| {
                    y.get(scan_x as usize)
                        .is_some_and(|c| c.content == CellContent::Bomb)
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
                if self
                    .board
                    .get(scan_y as usize)
                    .is_some_and(|y| y.get(scan_x as usize).is_some())
                {
                    array.push((scan_x as usize, scan_y as usize));
                }
            }
        }
        return array;
    }

    fn regenerate_if_not_empty(
        &mut self,
        click_pos: &BlockPos,
        player: Entity,
        layer: &mut ChunkLayer,
    ) -> bool {
        if self
            .board
            .flatten()
            .iter()
            .all(|e| e.state == CellState::Closed)
        {
            tracing::warn!(
                "minesweeper: a bomb or number was the first clicked cell. Generating new Board."
            );
            self.board = Self::generate_board(BOMB_AMT, self.board[0][0].pos);
            self.click_left(click_pos, player, layer);
            return true;
        }
        return false;
    }
}

impl<const DIM: usize> CustomGame for MineSweeperGame<DIM> {
    fn build_blocks(&mut self, layer: &mut ChunkLayer) -> Result<(), BuildGameError> {
        if (0..DIM).cartesian_product(0..DIM).any(|(x, y)| {
            layer
                .block(self.board[y][x].pos)
                .is_some_and(|b| !b.state.is_air())
        }) {
            return Err(BuildGameError::BlocksInTheWay);
        }
        for (x, y) in (0..DIM).cartesian_product(0..DIM) {
            let block = BlockState::MOSS_BLOCK;
            // let block = get_num_color(self.board[y][x].content);
            layer.set_block(self.board[y][x].pos, block);
        }
        self.is_build = true;
        return Ok(());
    }
    fn tick(&mut self, _layer: &mut ChunkLayer) {
        if !self.is_over {
            self.comp_time += 1;
        }
        if self.flag_lock > 0 {
            self.flag_lock -= 1;
        }
    }
    fn click_right(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer) {
        if self.flag_lock != 0 {
            return;
        }
        for (x, y) in (0..DIM).cartesian_product(0..DIM) {
            let pos = self.board[y][x].pos;
            if pos == *click_pos {
                self.flag_lock = 4;
                match self.board[y][x].state {
                    CellState::Closed => {
                        layer.set_block(pos, BlockState::RED_WOOL);
                        self.board[y][x].state = CellState::Flagged;
                    }
                    CellState::Flagged => {
                        layer.set_block(pos, BlockState::MOSS_BLOCK);
                        self.board[y][x].state = CellState::Closed;
                    }
                    _ => (),
                }
            }
        }
    }
    fn click_left(&mut self, click_pos: &BlockPos, player: Entity, layer: &mut ChunkLayer) {
        if self.is_over {
            self.should_despawn = true;
            return;
        }
        for (x, y) in (0..DIM).cartesian_product(0..DIM) {
            if self.board[y][x].pos == *click_pos {
                match self.board[y][x].state {
                    CellState::Closed => match self.board[y][x].content {
                        CellContent::Bomb => {
                            if self.regenerate_if_not_empty(click_pos, player, layer) {
                                return;
                            }
                            sound(layer, Sound::EntityGenericExplode, click_pos);
                            for x in 0..DIM {
                                for y in 0..DIM {
                                    let block = get_num_color(self.board[y][x].content);
                                    layer.set_block(self.board[y][x].pos, block);
                                }
                            }
                            self.is_over = true;
                        }
                        CellContent::Empty => {
                            sound(layer, Sound::EntityFrogStep, click_pos);
                            self.board[y][x].state = CellState::Opened;
                            let block = get_num_color(self.board[y][x].content);
                            layer.set_block(self.board[y][x].pos, block);
                            for adj in self.get_adjacent_cells((x, y)) {
                                let cell = self.board[adj.1][adj.0];
                                if cell.state == CellState::Closed {
                                    // simulate click on adjacent empty fields
                                    self.click_left(&cell.pos, player, layer);
                                }
                            }
                        }
                        CellContent::Number(_) => {
                            if self.regenerate_if_not_empty(click_pos, player, layer) {
                                return;
                            }
                            sound(layer, Sound::EntityFrogStep, click_pos);
                            self.board[y][x].state = CellState::Opened;
                            let b = layer
                                .set_block(*click_pos, get_num_color(self.board[y][x].content));
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
                if !self.is_over
                    && (0..DIM).cartesian_product(0..DIM).all(|(x, y)| {
                        self.board[y][x].state == CellState::Opened
                            || self.board[y][x].content == CellContent::Bomb
                    })
                {
                    sound(
                        layer,
                        Sound::ItemGoatHornSound1,
                        &self.board[DIM / 2][DIM / 2].pos,
                    );
                    self.is_over = true;
                    self.is_won = true;
                }
            }
        }
    }
    fn reset(&self, layer: &mut ChunkLayer, pgsql: &mut crate::postgres_wrapper::PostgresWrapper) {
        for x in 0..DIM {
            for y in 0..DIM {
                layer.set_block(self.board[y][x].pos, BlockState::AIR);
            }
        }

        if self.is_won {
            pgsql.insert_minesweeper(
                DIM as i32,
                2,
                self.comp_time as i32,
                BOMB_AMT as i32,
                self.player.1,
            );
        }
    }
    fn get_player(&self) -> (Entity, UniqueId) {
        self.player
    }
    fn should_despawn(&self) -> bool {
        self.should_despawn
    }
}
