// use std::sync::Arc;

// use postgres::{Client, NoTls};
// use valence::prelude::*;

// #[derive(Component)]
// struct PostgresWrapper(Arc<Client>);

// impl PostgresWrapper {
//     fn new() -> Self {
//         let c = Client::connect("host=localhost user=postgres", NoTls).unwrap();
//         PostgresWrapper(Box::new(c))
//     }

//     fn get_database(&mut self) -> &mut Client {
//         if self.0.is_closed() {
//             let c = Client::connect("host=localhost user=postgres", NoTls).unwrap();
//             *self.0 = c;
//         }
//         &mut self.0
//     }
// }
