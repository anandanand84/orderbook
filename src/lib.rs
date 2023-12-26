extern crate bincode;
extern crate prost;
extern crate stock_messages;
extern crate itertools;

mod book;

use std::{collections::{HashMap}, convert::TryFrom, cell::RefCell};
use stock_messages::stock_messages::SnapshotMessage;
extern crate wasm_bindgen;

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
        let book = OrderBook::new("", 0);
        BOOK_MAP.with(|map_ref| {
            let mut map = map_ref.borrow_mut();
            map.insert(book_id, book);
        });
        return true;
    }
}

#[wasm_bindgen]
pub fn update_book_level(book_id: u32, bytes: Vec<u8>) -> bool {
    return BOOK_MAP.with(|map_ref| {
        let mut map = map_ref.borrow_mut();
        let book = map.get_mut(&book_id);
        if let Some(book) = book {
            return book.update_level(bytes);
        }
        return false;
    });
}

//use this only for testing
#[wasm_bindgen]
pub fn update_book_level_struct(book_id: u32, side:u32, price: f64, size: f64) -> bool {
    return BOOK_MAP.with(|map_ref| {
        let mut map = map_ref.borrow_mut();
        let book = map.get_mut(&book_id);
        if let Some(book) = book {
            return book.update_level_message(stock_messages::stock_messages::LevelUpdate { r#type: 0, exchange: "".to_string(), price: price, product_id: "".to_string(), sequence: (book.sequence + 1) as i32, side: side as i32, size: size, time: 0, count: 0 });
        }
        return false;
    });
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
pub fn get_grouped_snapshot(book_id: u32, count:usize) -> Vec<f64> {
    let mut out:Vec<f64> = Vec::new();
    let result = BOOK_MAP.with(|map_ref| {
        let map = map_ref.borrow();
        let book = map.get(&book_id);
        if let Some(book) = book {
            let snapshot = book.get_grouped_snapshot_new(count);
            for level in snapshot.asks.iter().rev() {
                out.push(level.price);
                out.push(level.total_size);
            }
            out.push(99999.99999);
            out.push(99999.99999);
            for level in snapshot.bids.iter() {
                out.push(level.price);
                out.push(level.total_size);
            }
        }
    });
    return out;
}


#[wasm_bindgen]
pub fn set_group_size(book_id: u32, size: f64) {
    let result = BOOK_MAP.with(|map_ref| {
        let mut map = map_ref.borrow_mut();
        let book = map.get_mut(&book_id);
        if let Some(orderbook) = book {
            orderbook.set_group_size(size);
        };
    });
    return result;
}

#[wasm_bindgen]
pub fn sum(a:u64, b:u64) -> u64 {
    return a + b
}

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"Starte consoel...".into());
    eprintln!("Started...");
}