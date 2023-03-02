extern crate bincode;
extern crate prost;
extern crate stock_messages;
extern crate itertools;

mod book;

use std::{collections::{HashMap}, convert::TryFrom, cell::RefCell};
use stock_messages::stock_messages::SnapshotMessage;
use wasm_bindgen::prelude::*;

pub use book::book::{OrderBook, OrderType, Level, OrderBookSnapshot};

thread_local! {
    static BOOK_MAP: RefCell<HashMap<String, OrderBook>> = RefCell::new(HashMap::new());
}

#[wasm_bindgen]
pub fn create_book(bytes: Vec<u8>) -> bool {
    let new_book = OrderBook::try_from(bytes);
    if let Ok(book) = new_book {
        BOOK_MAP.with(|map_ref| {
            let mut map = map_ref.borrow_mut();
            map.insert(book.instrument.clone(), book);
        });
        return true;
    } else {
        return false;
    }
}

#[wasm_bindgen]
pub fn has_book(instrument: String) -> bool {
    let result = BOOK_MAP.with(|map_ref| {
        let map = map_ref.borrow();
        let book = map.get(&instrument);
        book.is_some()
    });
    return result;
}

#[wasm_bindgen]
pub fn get_snapshot(instrument: String) -> Vec<u8> {
    let result = BOOK_MAP.with(|map_ref| {
        let map = map_ref.borrow();
        let book = map.get(&instrument);
        book.map_or(Vec::new(), |book| book.clone().into())
    });
    return result;
}

#[wasm_bindgen]
pub fn sum(a:u64, b:u64) -> u64 {
    return a + b
}