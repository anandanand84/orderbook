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
    static BOOK_MAP: RefCell<HashMap<u32, OrderBook>> = RefCell::new(HashMap::new());
}

#[wasm_bindgen]
pub fn update_snapshot(book_id:u32, bytes: Vec<u8>) -> bool {
    let new_book: Result<OrderBook, &str> = OrderBook::try_from(bytes);
    if let Ok(book) = new_book {
        BOOK_MAP.with(|map_ref| {
            let mut map = map_ref.borrow_mut();
            map.insert(book_id, book);
        });
        return true;
    } else {
        return false;
    }
}

#[wasm_bindgen]
pub fn update_book_level(book_id: u32, bytes: Vec<u8>) -> bool {
    BOOK_MAP.with(|map_ref| {
        let mut map = map_ref.borrow_mut();
        let book = map.get_mut(&book_id);
        if let Some(book) = book {
            return book.update_level(bytes);
        }
        return false;
    });
    return false;
}

#[wasm_bindgen]
pub fn has_book(book_id: u32) -> bool {
    let result = BOOK_MAP.with(|map_ref| {
        let map = map_ref.borrow();
        let book = map.get(&book_id);
        book.is_some()
    });
    return result;
}

#[wasm_bindgen]
pub fn get_snapshot(book_id: u32) -> Vec<u8> {
    let result = BOOK_MAP.with(|map_ref| {
        let map = map_ref.borrow();
        let book = map.get(&book_id);
        book.map_or(Vec::new(), |book| book.into())
    });
    return result;
}

#[wasm_bindgen]
pub fn sum(a:u64, b:u64) -> u64 {
    return a + b
}