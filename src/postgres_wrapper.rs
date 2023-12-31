use std::{sync::Mutex, time::SystemTime};
use valence::prelude::*;

use postgres::{Client, NoTls};

const CONNECT_STR: &str = "host=localhost user=postgres";

//The Mutex can be removed as soon as bevy 0.12 is being used for valence,
//as that will remove the need for Locals to be Sync
pub struct PostgresWrapper(pub Mutex<Client>);

impl Default for PostgresWrapper {
    fn default() -> Self {
        PostgresWrapper(Mutex::new(
            postgres::Client::connect(CONNECT_STR, NoTls).unwrap(),
        ))
    }
}

impl PostgresWrapper {
    pub fn get_highest_streak(&self, uuid: &UniqueId) -> Option<i32> {
        self.check_connection();

        let mut db_conn = self.0.lock().unwrap();
        match db_conn.query_one(
            "SELECT MAX(streak) FROM rsg_games WHERE player_uuid = $1",
            &[&uuid.as_bytes().as_ref()],
        ) {
            Ok(row) => return row.get(0),
            Err(err) => {
                tracing::warn!("A players highest streak couldnt be loaded. {}", err);
                return None;
            }
        }
    }

    pub fn get_minesweeper_fastest(&self, uuid: &UniqueId) -> Option<(i32, i32, i32)> {
        self.check_connection();

        let mut db_conn = self.0.lock().unwrap();
        match db_conn.query_one(
            "SELECT size, dim, MIN(comp_time)
            FROM minesweeper_games WHERE (player_uuid = $1)
            GROUP BY size, dim",
            &[&uuid.as_bytes().as_ref()],
        ) {
            Ok(row) => return Some((row.get(0), row.get(1), row.get(2))),
            Err(err) => {
                tracing::warn!("A players best time couldnt be loaded. {}", err);
                return None;
            }
        }
    }

    pub fn insert_rsg(&self, dim: i32, streak: i32, uuid: UniqueId) {
        self.check_connection();

        let time = SystemTime::now();
        match self.0.lock().unwrap().execute(
            "INSERT INTO rsg_games (date, size, streak, player_uuid) VALUES ($1, $2, $3, $4)",
            &[&time, &dim, &streak, &uuid.as_bytes().as_ref()],
        ) {
            Ok(i) => {
                if i != 1 {
                    tracing::error!("Wrong number of database Entries modified.")
                } else {
                    tracing::debug!("new database entry saved. (rsg)");
                }
            }
            Err(err) => tracing::error!("Couldnt save data into Database {}", err),
        }
    }

    pub fn insert_minesweeper(
        &self,
        size: i32,
        dimension: i32,
        comp_time: i32,
        bomb_amt: i32,
        uuid: UniqueId,
    ) {
        self.check_connection();

        let time = SystemTime::now();
        match self.0.lock().unwrap().execute(
            "INSERT INTO minesweeper_games (date, size, dim, comp_time, bomb_amt, player_uuid) VALUES ($1, $2, $3, $4, $5, $6)",
            &[&time, &size, &dimension, &comp_time, &bomb_amt, &uuid.as_bytes().as_ref()],
        ) {
            Ok(i) => {
                if i != 1 {
                    tracing::error!("Wrong number of database Entries modified.")
                } else {
                    tracing::debug!("new database entry saved. (minesweeper)");
                }
            }
            Err(err) => tracing::error!("Couldnt save data into Database {}", err),
        }
    }

    fn check_connection(&self) {
        let mut db_conn = self.0.lock().unwrap();
        if db_conn.is_closed() {
            tracing::info!("The postgres connection has closed, opening a new one.");
            *db_conn = postgres::Client::connect(CONNECT_STR, NoTls).unwrap();
        }
    }
}
