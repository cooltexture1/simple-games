use valence::{
    nbt::{compound, Value},
    prelude::*,
};

pub enum StartItemType {
    RSG5,
    RSG7,
    Minesweeper,
}

impl StartItemType {
    pub fn all_types() -> impl IntoIterator<Item = StartItemType> {
        [
            StartItemType::RSG5,
            StartItemType::RSG7,
            StartItemType::Minesweeper,
        ]
    }

    pub fn get_start_item_type(item: &ItemStack) -> Option<StartItemType> {
        if item.item != ItemKind::Stick {
            return None;
        }
        if let Value::Compound(c) = item.nbt.clone()?.get("display")? {
            if let Value::String(s) = c.get("Name")? {
                match s.as_str() {
                    "\"Repeat Sequence 5x5\"" => Some(StartItemType::RSG5),
                    "\"Repeat Sequence 7x7\"" => Some(StartItemType::RSG7),
                    "\"Minesweeper\"" => Some(StartItemType::Minesweeper),
                    _ => None,
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
        //TODO test this
    }

    pub fn create_start_item(item_type: StartItemType) -> ItemStack {
        match item_type {
            StartItemType::RSG5 => ItemStack::new(
                ItemKind::Stick,
                1,
                Some(compound! {
                    "display" => compound! {
                        "Name" => "\"Repeat Sequence 5x5\"",
                    }
                }),
            ),
            StartItemType::RSG7 => ItemStack::new(
                ItemKind::Stick,
                1,
                Some(compound! {
                    "display" => compound! {
                        "Name" => "\"Repeat Sequence 7x7\"",
                    }
                }),
            ),
            StartItemType::Minesweeper => ItemStack::new(
                ItemKind::Stick,
                1,
                Some(compound! {
                    "display" => compound! {
                        "Name" => "\"Minesweeper\"",
                    }
                }),
            ),
        }
    }
}
