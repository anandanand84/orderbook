extern crate bincode;
extern crate prost;
extern crate stock_messages;
extern crate itertools;

mod book;

use std::collections::{HashMap};

#[macro_use]
extern crate lazy_static;

pub use book::book::{OrderBook, OrderType, Level, OrderBookSnapshot};

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
