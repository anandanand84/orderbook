extern crate bincode;
extern crate prost;
extern crate stock_messages;

mod book;

use book::book::{OrderBook, OrderType};
use bytes::{Bytes, BytesMut};
use stock_messages::stock_messages::{SnapshotMessage, Side, LevelUpdate};
use prost::Message;
use std::collections::{HashMap};

#[macro_use]
extern crate lazy_static;


lazy_static! {
    pub static ref BOOK_MAP: HashMap<String, OrderBook> = {
        let m = HashMap::new();
        m
    };
}

// pub fn create_book(instrument: String, bytes: ) -> bool {
//     let booK_res = BOOK_MAP.get(&instrument);
//     match booK_res {
//         Some(_) => true,
//         None => false
//     }
// }

// pub fn has_book(instrument: String) -> bool {
//     let booK_res = BOOK_MAP.get(&instrument);
//     match booK_res {
//         Some(_) => true,
//         None => false
//     }
// }


// pub fn get_snapshot(instrument: String) {

// }
