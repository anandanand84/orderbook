extern crate serde;
extern crate stock_messages;

#[cfg(not(target_arch = "wasm32"))]
extern crate colored;

pub mod book {
    use crate::book_utils::book::group;
    use crate::book_utils::book::value_to_scale;
    use crate::itertools::Itertools;
    use bigdecimal::BigDecimal;
    use bigdecimal::RoundingMode;
    use num_traits::cast::ToPrimitive;
    use num_traits::identities::Zero;
    use num_traits::FromPrimitive;
    use prost::Message;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use std::convert::TryFrom;
    use std::convert::TryInto;
    use std::ops::Bound::Included;
    use std::ops::{Add, Mul, Sub};
    use std::time::SystemTime;
    use stock_messages::stock_messages::{
        BookInfo, LevelUpdate, PriceLevel, Side, SnapshotMessage, Type,
    };

    // #[cached(
    //     type = "SizedCache<String, f64>",
    //     create = "{ SizedCache::with_size(100000) }",
    //     convert = r#"{ format!("{}{}{}", price, group_size, group_lower) }"#
    // )]
    // pub fn group_decimal(price: f64, group_size: f64, group_lower: bool) -> f64 {
    //     let group = (price / group_size) as u64;
    //     let mut current_price = (group as f64) * group_size;
    //     if !group_lower && (price > current_price) {
    //         current_price += group_size;
    //     }
    //     let decimal_places = count_decimal_places(group_size);
    //     BigDecimal::from_f64(current_price).unwrap_or_default()
    //         .with_scale(decimal_places as i64)
    //         .to_f64()
    //         .unwrap_or(0f64)
    // }

    use std::cmp::max;

    // #[cached(
    //     type = "SizedCache<String, BigDecimal>",
    //     create = "{ SizedCache::with_size(100000) }",
    //     convert = r#"{ format!("{}{}{}", price, group_size, group_lower) }"#
    // )]
    // pub fn group_decimal_bigdecimal(price: f64, group_size: f64, group_lower: bool) -> BigDecimal {
    //     let group = (price / group_size) as u64;
    //     let mut current_price = (group as f64) * group_size;
    //     if !group_lower && (price > current_price) {
    //         current_price += group_size;
    //     }
    //     let decimal_places = count_decimal_places(group_size);
    //     BigDecimal::from_f64(current_price).unwrap_or_default().with_scale(decimal_places as i64)
    // }

    // fn count_decimal_places(num: f64) -> usize {
    //     let decimal_str = format!("{:.20}", num - num.floor());
    //     decimal_str.trim_end_matches('0').len() - 2
    // }

    pub type Price = BigDecimal;
    pub type Size = BigDecimal;
    pub type Value = BigDecimal;

    #[derive(Debug, Hash, Eq, PartialEq, Clone)]
    pub struct Level {
        pub price: Price,
        pub size: Size,
        pub value: Value,
    }

    pub enum OrderType {
        Bid = 1,
        Ask = 2,
    }

    impl Level {
        pub fn new(price: f64, size: f64) -> Level {
            Level {
                price: BigDecimal::from_f64(price).unwrap_or_default(),
                size: BigDecimal::from_f64(size).unwrap_or_default(),
                value: BigDecimal::from_f64(price).unwrap_or_default().mul(BigDecimal::from_f64(size).unwrap_or_default()),
            }
        }
    }

    impl From<PriceLevel> for Level {
        fn from(level: PriceLevel) -> Self {
            Level::new(level.price, level.total_size)
        }
    }

    impl Into<PriceLevel> for Level {
        fn into(self) -> PriceLevel {
            PriceLevel {
                price: self.price.clone().to_string().parse().unwrap_or(0f64),
                total_size: self.size.clone().to_string().parse().unwrap_or(0f64),
                total_value: BigDecimal::from(self.price)
                    .mul(BigDecimal::from(self.size))
                    .clone()
                    .to_string()
                    .parse()
                    .unwrap_or(0f64),
            }
        }
    }

    impl Into<SnapshotLevel> for Level {
        fn into(self) -> SnapshotLevel {
            SnapshotLevel {
                price: self.price.clone().to_string().parse().unwrap_or(0f64),
                total_size: self.size.clone().to_string().parse().unwrap_or(0f64),
                total_value: BigDecimal::from(self.price)
                    .mul(BigDecimal::from(self.size))
                    .clone()
                    .to_string()
                    .parse()
                    .unwrap_or(0f64),
                relative_size: 0,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct OrderBook {
        pub instrument: String,
        pub sequence: u64,
        pub bids: BTreeMap<Price, Level>,
        pub asks: BTreeMap<Price, Level>,

        pub bids_total: Size,
        pub bids_value_total: Value,
        pub asks_total: Size,
        pub asks_value_total: Value,

        pub grouped_bids: BTreeMap<Price, Size>,
        pub grouped_asks: BTreeMap<Price, Size>,
        group_size: f64, // orderPool: OrderPool = {};
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct SnapshotLevel {
        pub relative_size: i64,
        pub price: f64,
        pub total_size: f64,
        pub total_value: f64,
    }

    impl SnapshotLevel {
        pub fn new() -> SnapshotLevel {
            SnapshotLevel {
                price: 0.0,
                total_size: 0.0,
                total_value: 0.0,
                relative_size: 0,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct OrderBookInfo {
        pub asks_total: f64,
        pub asks_value_total: f64,
        pub bids_value_total: f64,
        pub bids_total: f64,
        pub spread: String,
        pub sequence: u64,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct OrderBookSnapshot {
        pub instrument: String, //"Bittrex:ETH/USDT"
        pub time: u64,          //"2017-10-14T20:08:50.920Z"
        pub info: OrderBookInfo,
        pub asks: Vec<SnapshotLevel>,
        pub bids: Vec<SnapshotLevel>,
        pub cum_bid_values: Vec<SnapshotLevel>,
        pub cum_ask_values: Vec<SnapshotLevel>,
    }

    impl From<SnapshotMessage> for OrderBook {
        fn from(snapshot: SnapshotMessage) -> Self {
            let sequence: u64 = snapshot.source_sequence.try_into().unwrap_or(0u64);
            let mut book = OrderBook::new(&snapshot.product_id, sequence);
            book.bids_total = snapshot
                .bids
                .iter()
                .fold(BigDecimal::zero(), |total, current| {
                    total.add(BigDecimal::from_f64(current.price).unwrap_or_default())
                });
            book.asks_total = snapshot
                .asks
                .iter()
                .fold(BigDecimal::zero(), |total, current| {
                    total.add(BigDecimal::from_f64(current.price).unwrap_or_default())
                });
            book.bids_value_total =
                snapshot
                    .bids
                    .iter()
                    .fold(BigDecimal::zero(), |total, current| {
                        total.add(
                            BigDecimal::from_f64(current.price).unwrap_or_default()
                                .mul(BigDecimal::from_f64(current.total_size).unwrap_or_default()),
                        )
                    });
            book.asks_value_total =
                snapshot
                    .asks
                    .iter()
                    .fold(BigDecimal::zero(), |total, current| {
                        total.add(
                            BigDecimal::from_f64(current.price).unwrap_or_default()
                                .mul(BigDecimal::from_f64(current.total_size).unwrap_or_default()),
                        )
                    });
            book.asks = snapshot
                .asks
                .into_iter()
                .map(|pricelevel| (BigDecimal::from_f64(pricelevel.price).unwrap_or_default(), Level::from(pricelevel)))
                .collect();
            book.bids = snapshot
                .bids
                .into_iter()
                .map(|pricelevel| (BigDecimal::from_f64(pricelevel.price).unwrap_or_default(), Level::from(pricelevel)))
                .collect();
            book.refresh_groupings();
            book
        }
    }

    impl TryFrom<Vec<u8>> for OrderBook {
        type Error = &'static str;
        fn try_from(buf: Vec<u8>) -> Result<Self, Self::Error> {
            let snapshot_decode = SnapshotMessage::decode(buf);
            match snapshot_decode {
                Ok(snapshot) => Ok(OrderBook::from(snapshot)),
                Err(_) => Err("Failed to decode the snapshot"),
            }
        }
    }

    impl Into<Vec<u8>> for &OrderBook {
        fn into(self) -> Vec<u8> {
            let info = BookInfo {
                sequence: self.sequence as u32,
                ask_total_size: self.asks_total.to_string().parse().unwrap_or(0f64),
                ask_total_value: self.asks_value_total.to_string().parse().unwrap_or(0f64),
                bid_total_size: self.bids_total.to_string().parse().unwrap_or(0f64),
                bid_tota_value: self.bids_value_total.to_string().parse().unwrap_or(0f64),
            };
            let message = SnapshotMessage {
                trades: vec![],
                r#type: Type::Snapshot.into(),
                exchange: -1,
                info: info,
                product_id: String::from(&self.instrument),
                bids: self
                    .bids
                    .iter()
                    .map(|(_, level)| level.clone())
                    .map(|y| y.clone().into())
                    .collect(),
                asks: self
                    .asks
                    .iter()
                    .map(|(_, level)| level.clone())
                    .map(|y| y.clone().into())
                    .collect(),
                source_sequence: self.sequence as i32,
                takers: vec![],
                time: SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            };
            let mut buf: Vec<u8> = Vec::new();
            message.encode(&mut buf).unwrap();
            buf
        }
    }

    impl Into<SnapshotMessage> for OrderBook {
        fn into(self: OrderBook) -> SnapshotMessage {
            let info = BookInfo {
                sequence: self.sequence as u32,
                ask_total_size: self.asks_total.to_string().parse().unwrap_or(0f64),
                ask_total_value: self.asks_value_total.to_string().parse().unwrap_or(0f64),
                bid_total_size: self.bids_total.to_string().parse().unwrap_or(0f64),
                bid_tota_value: self.bids_value_total.to_string().parse().unwrap_or(0f64),
            };
            let message = SnapshotMessage {
                trades: vec![],
                r#type: Type::Snapshot.into(),
                exchange: -1,
                info: info,
                product_id: String::from(&self.instrument),
                bids: self
                    .bids
                    .iter()
                    .map(|(_, level)| level.clone())
                    .map(|y| y.clone().into())
                    .collect(),
                asks: self
                    .asks
                    .iter()
                    .map(|(_, level)| level.clone())
                    .map(|y| y.clone().into())
                    .collect(),
                source_sequence: self.sequence as i32,
                takers: vec![],
                time: 100u64,
            };
            message
        }
    }

    impl OrderBook {
        #[cfg(not(target_arch = "wasm32"))]
        pub fn print_debug(&self) {
            use colored::*;

            for (price, ask) in self.asks.iter().take(100).rev() {
                println!(
                    "{} {} {}",
                    ask.price.to_string().red(),
                    ask.size.to_string().red(),
                    ask.value.to_string().red()
                );
            }
            println!("---------------------");
            for (price, bid) in self.bids.iter().rev().take(100) {
                println!(
                    "{} {} {}",
                    bid.price.to_string().green(),
                    bid.size.to_string().green(),
                    bid.value.to_string().green()
                );
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        pub fn print_grouped_debug(&self) {
            use colored::*;
            println!("Group Size: {}", self.group_size);
            for (price, size) in self.grouped_asks.iter().take(100).rev() {
                println!("{} {}", price.to_string().red(), size.to_string().red());
            }
            println!("---------------------");
            for (price, size) in self.grouped_bids.iter().rev().take(100) {
                println!("{} {}", price.to_string().green(), size.to_string().green());
            }
        }

        pub fn new(instrument: &str, sequence: u64) -> OrderBook {
            OrderBook {
                sequence: sequence,
                instrument: String::from(instrument),
                bids: BTreeMap::new(),
                asks: BTreeMap::new(),
                bids_total: BigDecimal::zero(),
                bids_value_total: BigDecimal::zero(),
                asks_total: BigDecimal::zero(),
                asks_value_total: BigDecimal::zero(),
                grouped_bids: BTreeMap::new(),
                grouped_asks: BTreeMap::new(),
                group_size: 1.0,
            }
        }

        pub fn set_group_size(&mut self, group_size: f64) {
            self.group_size = group_size;
            self.refresh_groupings();
        }

        pub fn refresh_groupings(&mut self) {
            let group_size = self.group_size;
            let mut grouped_bids = BTreeMap::new();
            let mut grouped_asks = BTreeMap::new();

            for (_, pricelevel) in &self.asks {
                let price = pricelevel.price.clone();
                let size = pricelevel.size.clone();
                let group_price = group(price, group_size, false);
                let existing_size = grouped_asks
                    .entry(group_price)
                    .or_insert(BigDecimal::zero());
                *existing_size += size;
            }

            for (_, pricelevel) in &self.bids {
                let price = pricelevel.price.clone();
                let size = pricelevel.size.clone();
                let group_price = group(price, group_size, true);
                let existing_size = grouped_bids
                    .entry(group_price)
                    .or_insert(BigDecimal::zero());
                *existing_size += size;
            }

            self.grouped_bids = grouped_bids;
            self.grouped_asks = grouped_asks;
        }

        pub fn verify_sequence(&self, sequence: i32) -> (bool, bool) {
            let next_sequence = (self.sequence + 1) as i32;
            let received_sequence = sequence;
            if received_sequence < next_sequence {
                let result = format!(
                    "old sequenece for {} ignoring current received sequence: {} book sequence: {}",
                    self.instrument, received_sequence, next_sequence
                );
                #[cfg(feature = "console_error_panic_hook")]
                web_sys::console::error_1(&result.clone().into());
                println!("{}", result);
                return (true, true);
            } else if received_sequence > next_sequence {
                let result = format!(
                    "SEQUENCE MISMATCH {} received {}, next {}",
                    self.instrument, received_sequence, next_sequence
                );
                #[cfg(feature = "console_error_panic_hook")]
                web_sys::console::error_1(&result.clone().into());
                println!("{}", result);
                return (true, false);
            }
            return (false, false);
        }

        pub fn update_level_message(&mut self, level_message: LevelUpdate) -> bool {
            let (stop, valid) = self.verify_sequence(level_message.sequence);
            if stop {
                return valid;
            }
            let side = Side::from_i32(level_message.side).unwrap();
            let order_type = if side == Side::Buy {
                OrderType::Bid
            } else {
                OrderType::Ask
            };
            if level_message.size == 0.0 {
                self.remove_level(
                    OrderType::Bid,
                    level_message.price,
                    level_message.sequence as u64,
                );
                self.remove_level(
                    OrderType::Ask,
                    level_message.price,
                    level_message.sequence as u64,
                );
                // self.refresh_groupings();
                return true;
            } else {
                self.add_level(
                    order_type,
                    level_message.price,
                    level_message.size,
                    level_message.sequence as u64,
                );
                // self.refresh_groupings();
                return true;
            }
        }

        pub fn update_level(&mut self, bytes: Vec<u8>) -> bool {
            let level_message: LevelUpdate = LevelUpdate::decode(bytes).unwrap();
            return self.update_level_message(level_message);
        }

        // This is update
        pub fn add_level(
            &mut self,
            order_type: OrderType,
            price: f64,
            size: f64,
            sequence: u64,
        ) -> bool {
            self.sequence = sequence;
            let group_size = self.group_size; // change this to your desired group size
            match order_type {
                OrderType::Bid => {
                    self.bids_total = self.bids_total.clone().add(BigDecimal::from_f64(size).unwrap_or_default());
                    self.bids_value_total = self
                        .bids_value_total
                        .clone()
                        .add(BigDecimal::from_f64(size).unwrap_or_default().mul(BigDecimal::from_f64(price).unwrap_or_default()));
                    let current_size_at_level = self
                        .bids
                        .get(&BigDecimal::from_f64(price).unwrap_or_default())
                        .map(|x| x.size.clone())
                        .unwrap_or(BigDecimal::zero());
                    let price_decimal = BigDecimal::from_f64(price).unwrap_or_default();
                    self.bids
                        .insert(BigDecimal::from_f64(price).unwrap_or_default(), Level::new(price, size));
                    let group_price = group(price_decimal, group_size, true);
                    let bids_group = self
                        .grouped_bids
                        .entry(group_price)
                        .or_insert(BigDecimal::zero());
                    *bids_group -= current_size_at_level;
                    *bids_group += BigDecimal::from_f64(size).unwrap_or_default();
                }
                OrderType::Ask => {
                    self.asks_total = self.asks_total.clone().add(BigDecimal::from_f64(size).unwrap_or_default());
                    self.asks_value_total = self
                        .asks_value_total
                        .clone()
                        .add(BigDecimal::from_f64(size).unwrap_or_default().mul(BigDecimal::from_f64(price).unwrap_or_default()));
                    let current_size_at_level = self
                        .asks
                        .get(&BigDecimal::from_f64(price).unwrap_or_default())
                        .map(|x| x.size.clone())
                        .unwrap_or(BigDecimal::zero());
                    let price_decimal = BigDecimal::from_f64(price).unwrap_or_default();
                    self.asks
                        .insert(price_decimal.clone(), Level::new(price, size));
                    let group_price = group(price_decimal, group_size, false);
                    let asks_group = self
                        .grouped_asks
                        .entry(group_price)
                        .or_insert(BigDecimal::zero());
                    *asks_group -= current_size_at_level;
                    *asks_group += BigDecimal::from_f64(size).unwrap_or_default();
                }
            }
            true
        }

        pub fn remove_level(&mut self, order_type: OrderType, price: f64, sequence: u64) -> bool {
            self.sequence = sequence;
            let price_decimal = BigDecimal::from_f64(price).unwrap_or_default();
            match order_type {
                OrderType::Bid => {
                    let removed_level = self.bids.remove(&price_decimal);
                    if let Some(level) = removed_level {
                        self.bids_total = self.bids_total.clone().sub(level.size.clone());
                        self.bids_value_total = self.bids_value_total.clone().sub(level.value);
                        let group_size = self.group_size; // replace with your desired group size
                        let price_decimal = BigDecimal::from_f64(price).unwrap_or_default();
                        let grouped_price = group(price_decimal, group_size, true);
                        let removed_size = level.size;
                        let grouped_level = self.grouped_bids.entry(grouped_price);
                        match grouped_level {
                            std::collections::btree_map::Entry::Vacant(_) => {}
                            std::collections::btree_map::Entry::Occupied(level) => {
                                let existing_size = level.get();
                                if existing_size == &removed_size {
                                    level.remove_entry();
                                } else {
                                    *level.into_mut() -= removed_size;
                                }
                            }
                        }
                    }
                }
                OrderType::Ask => {
                    let removed_level = self.asks.remove(&price_decimal);
                    if let Some(level) = removed_level {
                        self.asks_total = self.asks_total.clone().sub(level.size.clone());
                        self.asks_value_total = self.asks_value_total.clone().sub(level.value);

                        let group_size = self.group_size; // replace with your desired group size
                        let price_decimal = BigDecimal::from_f64(price).unwrap_or_default();
                        let grouped_price = group(price_decimal, group_size, false);
                        let removed_size = level.size;
                        let grouped_level = self.grouped_asks.entry(grouped_price);
                        match grouped_level {
                            std::collections::btree_map::Entry::Vacant(_) => {}
                            std::collections::btree_map::Entry::Occupied(level) => {
                                let existing_size = level.get();
                                if existing_size == &removed_size {
                                    level.remove_entry();
                                } else {
                                    *level.into_mut() -= removed_size;
                                }
                            }
                        }
                    }
                }
            }
            return true;
        }
        pub fn get_levels(&self, count: i32) -> (Vec<Level>, Vec<Level>) {
            let asks = self
                .asks
                .iter()
                .take(count as usize)
                .map(|(x, y)| (x.clone(), y.clone()))
                .collect::<BTreeMap<Price, Level>>()
                .iter()
                .rev()
                .map(|(_, level)| level.clone())
                .collect::<Vec<Level>>();

            let bids = self
                .bids
                .iter()
                .rev()
                .take(count as usize)
                .map(|(_, level)| level.clone())
                .collect::<Vec<Level>>();

            (bids, asks)
        }

        pub fn get_heatmap_snapshot_levels(&self, total_count: usize, step_percent: f64) -> Vec<f64> {
            let highest_bid = self.bids.keys().next_back().cloned().unwrap_or_else(|| BigDecimal::from(0));
            let lowest_ask = self.asks.keys().next().cloned().unwrap_or_else(|| BigDecimal::from(0));
            let mid = (highest_bid + lowest_ask) / BigDecimal::from(2);
    
            // Calculate percentage range bounds
            let percent_step = BigDecimal::from_f64(step_percent / 100.0).unwrap_or_default();
            let max_range = mid.clone() * percent_step.clone() * BigDecimal::from_f64(total_count as f64).unwrap_or_default();
    
            let bid_lower_bound = mid.clone() - max_range.clone();
            let ask_upper_bound = mid.clone() + max_range;
    
            // Prepare vectors for bid and ask sizes
            let mut bid_sizes: Vec<f64> = vec![0.0; total_count];
            let mut ask_sizes: Vec<f64> = vec![0.0; total_count];
    
            // Populate the bid_sizes and ask_sizes vectors
            for (price, level) in self.bids.range(..=bid_lower_bound).rev() {
                let distance = mid.clone() - price;
                let steps_away = (distance / (mid.clone() * percent_step.clone())).to_usize().unwrap_or(0);
                if steps_away < total_count {
                    bid_sizes[steps_away] += level.size.to_f64().unwrap_or(0.0);
                } else {
                    break; // Stop iterating once we exceed the range
                }
            }
    
            for (price, level) in self.asks.range(..=ask_upper_bound) {
                let distance = price - &mid;
                let steps_away = (distance / (mid.clone() * percent_step.clone())).to_usize().unwrap_or(0);
                if steps_away < total_count {
                    ask_sizes[steps_away] += level.size.to_f64().unwrap_or(0.0);
                } else {
                    break; // Stop iterating once we exceed the range
                }
            }
    
            // Combine bid and ask sizes into a single vector for the heatmap
            bid_sizes.reverse(); // Reverse bids to have them in the correct order from mid to outer
            bid_sizes.extend(ask_sizes);
    
            bid_sizes
        }

        pub fn get_grouped_levels(&self, count: i32) -> (Vec<Level>, Vec<Level>) {
            let asks = self
                .grouped_asks
                .iter()
                .take(count as usize)
                .map(|(x, y)| (x.clone(), y.clone()))
                .collect::<BTreeMap<Price, Size>>()
                .iter()
                .rev()
                .map(|(price, size)| Level {
                    price: price.clone(),
                    size: size.clone(),
                    value: price * size,
                })
                .collect::<Vec<Level>>();

            let bids = self
                .grouped_bids
                .iter()
                .rev()
                .take(count as usize)
                .map(|(price, size)| Level {
                    price: price.clone(),
                    size: size.clone(),
                    value: price * size,
                })
                .collect::<Vec<Level>>();

            (bids, asks)
        }

        pub fn get_spread_percent(&self) -> f64 {
            let bid: f64 = self.get_best_bid();
            let ask: f64 = self.get_best_ask();
            return ((ask - bid) / bid) * 100.0;
        }

        pub fn get_spread(&self) -> f64 {
            let bid: f64 = self.get_best_bid();
            let ask: f64 = self.get_best_ask();
            return ask - bid;
        }

        pub fn get_best_bid(&self) -> f64 {
            self.bids
                .keys()
                .rev()
                .take(1)
                .map(|bigdec| bigdec.to_f64().unwrap())
                .sum()
        }

        pub fn get_best_ask(&self) -> f64 {
            self.asks
                .keys()
                .take(1)
                .map(|bigdec| bigdec.to_f64().unwrap())
                .sum()
        }

        pub fn get_cumulative_value(
            &self,
            order_type: OrderType,
            start_value: f64,
            end_value: f64,
        ) -> Vec<SnapshotLevel> {
            let mut bid_cum_size: BigDecimal = BigDecimal::default();
            let mut bid_cum_value: BigDecimal = BigDecimal::default();
            let mut cum_bid_values = Vec::new();

            match order_type {
                OrderType::Bid => {
                    for (_, level) in self
                        .bids
                        .range((
                            Included(BigDecimal::from_f64(start_value).unwrap_or_default()),
                            Included(BigDecimal::from_f64(end_value).unwrap_or_default()),
                        ))
                        .rev()
                    {
                        bid_cum_size = bid_cum_size.add(level.size.clone());
                        bid_cum_value = bid_cum_value.add(level.value.clone());
                        cum_bid_values.push(SnapshotLevel {
                            price: level.price.to_string().parse().unwrap_or(0f64),
                            total_size: bid_cum_size.to_f64().unwrap(),
                            total_value: bid_cum_value.to_f64().unwrap(),
                            relative_size: 0,
                        });
                    }
                }
                OrderType::Ask => {
                    for (_, level) in self.asks.range((
                        Included(BigDecimal::from_f64(start_value).unwrap_or_default()),
                        Included(BigDecimal::from_f64(end_value).unwrap_or_default()),
                    )) {
                        bid_cum_size = bid_cum_size.add(level.size.clone());
                        bid_cum_value = bid_cum_value.add(level.value.clone());
                        cum_bid_values.push(SnapshotLevel {
                            price: level.price.to_string().parse().unwrap_or(0f64),
                            total_size: bid_cum_size.to_f64().unwrap(),
                            total_value: bid_cum_value.to_f64().unwrap(),
                            relative_size: 0,
                        });
                    }
                }
            }
            return cum_bid_values;
        }

        pub fn get_grouped_snapshot_new(&self, count: usize) -> OrderBookSnapshot {
            let zero = BigDecimal::zero();
            let two = BigDecimal::from_f32(2.0).unwrap();
            let one = BigDecimal::from_f32(1.0).unwrap();
            let percentage = BigDecimal::from_f32(10.0/100.0).unwrap();
            let best_bid = self.grouped_bids.keys().rev().next().unwrap_or(&zero);
            let best_ask = self.grouped_asks.keys().next().unwrap_or(&zero);
            let mid_price = (best_bid + best_ask) / two;

            let scale = value_to_scale(self.group_size);

            let mid_price_lower = group(mid_price.clone(), self.group_size, true);
            let mid_price_higher = group(mid_price.clone(), self.group_size, false);

            let mut min_bid = mid_price.clone() * (one.clone() - percentage.clone());
            let mut max_ask = mid_price.clone() * (one.clone() + percentage.clone());

            let grouping_decimal = BigDecimal::from_f64(self.group_size).unwrap_or_default();

            min_bid = group(min_bid, self.group_size, true);
            max_ask = group(max_ask, self.group_size, false);

            let mut bid_groups_to_include = Vec::with_capacity(count as usize);
            let mut ask_groups_to_include = Vec::with_capacity(count as usize);

            let mut bid = mid_price_lower;
            let mut ask = mid_price_higher;
            for _ in 0..count {
                    bid = bid.with_scale_round(scale, RoundingMode::HalfUp);
                    bid_groups_to_include.push(bid.clone());
                    bid -= &grouping_decimal;

                    ask = ask.with_scale_round(scale, RoundingMode::HalfUp);
                    ask_groups_to_include.push(ask.clone());
                    ask += &grouping_decimal;
            }

            // while max_bid <= mid_price_lower {
            //     bid_groups_to_include.push(max_bid.clone());
            //     max_bid += grouping_decimal.clone();
            //     max_bid = max_bid.with_scale_round(scale, RoundingMode::HalfUp);
            // }

            // while min_ask >= mid_price_higher {
            //     ask_groups_to_include.push(min_ask.clone());
            //     min_ask -= grouping_decimal.clone();
            //     min_ask = min_ask.with_scale_round(scale, RoundingMode::HalfUp);
            // }

            let bids = bid_groups_to_include.into_iter().rev()
                .map(|price| {
                    self.grouped_bids
                        .get(&price)
                        .map_or_else(|| SnapshotLevel {
                            price: price.to_f64().unwrap(),
                            total_size: 0.0,
                            total_value: 0.0,
                            relative_size: 0,
                        }, |size| SnapshotLevel {
                            price: price.to_f64().unwrap(),
                            total_size: size.to_f64().unwrap_or(0.0),
                            total_value: 0.0,
                            relative_size: 0,
                        })
                })
                .rev()
                .take(count)
                .collect::<Vec<SnapshotLevel>>();

            let asks = ask_groups_to_include.into_iter()
                .map(|price| {
                    self.grouped_asks
                        .get(&price)
                        .map_or_else(|| SnapshotLevel {
                            price: price.to_f64().unwrap(),
                            total_size: 0.0,
                            total_value: 0.0,
                            relative_size: 0,
                        }, |size| SnapshotLevel {
                            price: price.to_f64().unwrap(),
                            total_size: size.to_f64().unwrap_or(0.0),
                            total_value: 0.0,
                            relative_size: 0,
                        })
                })
                .take(count)
                .collect::<Vec<SnapshotLevel>>();

            OrderBookSnapshot {
                instrument: self.instrument.to_owned(),
                bids: bids,
                time: 0u64,
                asks: asks,
                info: OrderBookInfo {
                    bids_total: self.bids_total.to_f64().unwrap_or(0.0),
                    bids_value_total: self.bids_value_total.to_f64().unwrap_or(0.0),
                    asks_total: self.asks_total.to_f64().unwrap_or(0.0),
                    asks_value_total: self.asks_value_total.to_f64().unwrap_or(0.0),
                    spread: self.get_spread().to_string(),
                    sequence: self.sequence,
                },
                cum_ask_values: Vec::new(),
                cum_bid_values: Vec::new(),
            }
        }

        pub fn get_grouped_snapshot(&self, count: usize) -> OrderBookSnapshot {
            let group_size: f64 = self.group_size;
            let depth_map_percent = 0.0;
            let mut asks = self
                .asks
                .iter()
                .map(|(x, y)| {
                    let snapshot_level: SnapshotLevel = y.clone().into();
                    (x, snapshot_level)
                })
                .group_by(|level| group(level.0.clone(), group_size, false))
                .into_iter()
                .map(|(grouped_price, grouped_levels)| {
                    let mut level = SnapshotLevel::new();
                    level.price = grouped_price.to_f64().unwrap_or_default();
                    grouped_levels.fold(&mut level, |level, (_, current_level)| {
                        level.total_size = level.total_size + current_level.total_size;
                        level.total_value = level.total_value + current_level.total_value;
                        level
                    });
                    level.into()
                })
                .take(count as usize)
                .collect::<Vec<SnapshotLevel>>();

            let mut bids = self
                .bids
                .iter()
                .rev()
                .map(|(x, y)| {
                    let snapshot_level: SnapshotLevel = y.clone().into();
                    (x, snapshot_level)
                })
                .group_by(|level| group(level.0.clone(), group_size, true))
                .into_iter()
                .map(|(grouped_price, grouped_levels)| {
                    let mut level = SnapshotLevel::new();
                    level.price = grouped_price.to_f64().unwrap_or_default();
                    grouped_levels.fold(&mut level, |level, (_, current_level)| {
                        level.total_size = level.total_size + current_level.total_size;
                        level.total_value = level.total_value + current_level.total_value;
                        level
                    });
                    level.into()
                })
                .take(count as usize)
                .collect::<Vec<SnapshotLevel>>();

            let mut max_value = asks
                .iter()
                .chain(bids.iter())
                .map(|level| (level.total_value * 1000000000000.0) as u64)
                .max()
                .unwrap_or(u64::MAX);

            max_value = max_value / 1000000000000;

            asks.iter_mut().for_each(|level| {
                level.relative_size = ((level.total_value / max_value as f64) * 38.0) as i64;
            });

            bids.iter_mut().for_each(|level| {
                level.relative_size = ((level.total_value / max_value as f64) * 38.0) as i64;
            });

            let mid_value = (self.get_best_ask() + self.get_best_bid()) / 2.0;

            let bid_bound = mid_value * (1.0 - (depth_map_percent as f64) / 100.0);

            let ask_bound = mid_value * (1.0 + (depth_map_percent as f64) / 100.0);

            // for(key, value) in self.asks.range((Included(BigDecimal::from(mid_value)), Included(BigDecimal::from(ask_bound)))) {
            //     println!("&key, &value, {:?} {:?}", key.clone(), value.clone());
            // }

            let spread = self.get_spread_percent();

            OrderBookSnapshot {
                instrument: self.instrument.to_owned(),
                bids: bids,
                time: 0u64,
                asks: asks,
                info: OrderBookInfo {
                    bids_total: self.bids_total.to_f64().unwrap_or(0.0),
                    bids_value_total: self.bids_value_total.to_f64().unwrap_or(0.0),
                    asks_total: self.asks_total.to_f64().unwrap_or(0.0),
                    asks_value_total: self.asks_value_total.to_f64().unwrap_or(0.0),
                    spread: spread.to_string(),
                    sequence: self.sequence,
                },
                cum_ask_values: if depth_map_percent == 0.0 {
                    Vec::new()
                } else {
                    self.get_cumulative_value(OrderType::Ask, mid_value, ask_bound)
                },
                cum_bid_values: if depth_map_percent == 0.0 {
                    Vec::new()
                } else {
                    self.get_cumulative_value(OrderType::Bid, bid_bound, mid_value)
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::book_utils::book::group;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::book::{Level, OrderBook, OrderType};
    use bigdecimal::BigDecimal;
    use bytes::BytesMut;
    use num_traits::FromPrimitive;
    use num_traits::cast::ToPrimitive;
    use prost::Message;
    use std::convert::TryInto;
    use std::error::Error;
    use stock_messages::stock_messages::SnapshotMessage;

    fn create_asks(book: &mut OrderBook) {
        (100..200)
            .into_iter()
            .zip((100..200).into_iter())
            .for_each(|(price, size)| {
                let sequence = book.sequence + 1;
                book.add_level(OrderType::Ask, price as f64, size as f64, sequence);
            })
    }

    fn create_bids(book: &mut OrderBook) {
        (1..100)
            .into_iter()
            .zip((1..100).into_iter())
            .for_each(|(price, size)| {
                let sequence = book.sequence + 1;
                book.add_level(OrderType::Bid, price as f64, size as f64, sequence);
            })
    }

    #[test]
    fn test_group_decimal_1() {
        let cases = vec![
            //value, grouping, result,
            (4.324210, 0.5, 4.00, true),
            (4.324210, 0.05, 4.3, true),
            (4.324210, 0.005, 4.32, true),
            (4.624210, 0.5, 4.5, true),
            (4.624210, 5.0, 0.0, true),
            (4.324210, 0.5, 4.50, false),
            (4.324210, 0.05, 4.35, false),
            (4.324210, 0.005, 4.325, false),
            (4.624210, 0.5, 5.0, false),
            (4.624210, 5.0, 5.0, false),
        ];

        for case in &cases {
            let a = BigDecimal::from_f64(case.0).unwrap_or_default();
            let b = BigDecimal::from_f64(case.1).unwrap_or_default();
            let c = BigDecimal::from_f64(case.2).unwrap_or_default();
            let q = group(a, case.1, case.3);
            assert_eq!(q, c);
        }
    }

    #[test]
    fn test_create_book() {
        let mut book = OrderBook::new("instrument", 100);
        create_asks(&mut book);
        create_bids(&mut book);
        // print_book(book, 50 as usize);
    }

    fn get_first_ask_and_bid(book: &OrderBook) -> ((f64, f64), (f64, f64)) {
        let first_ask = book
            .asks
            .values()
            .take(1)
            .map(|level| (level.price.to_f64().unwrap(), level.size.to_f64().unwrap()))
            .collect::<Vec<(f64, f64)>>()[0];
        let first_bid = book
            .bids
            .values()
            .rev()
            .take(1)
            .map(|level| (level.price.to_f64().unwrap(), level.size.to_f64().unwrap()))
            .collect::<Vec<(f64, f64)>>()[0];
        return (first_ask, first_bid);
    }

    #[test]

    fn test_grouped_snapshot() {
        let mut book = OrderBook::new("instrument", 100);
        create_asks(&mut book);
        create_bids(&mut book);
    }

    #[test]
    fn test_update_level() {
        let mut book = OrderBook::new("instrument", 100);
        create_asks(&mut book);
        create_bids(&mut book);

        //Verify initial state
        let (first_ask, first_bid) = get_first_ask_and_bid(&book);

        assert_eq!(first_ask.0, 100.0);
        assert_eq!(first_ask.1, 100.0);
        assert_eq!(first_bid.0, 99.0);
        assert_eq!(first_bid.1, 99.0);

        // ADD NEW LEVEL
        let mut sequence = book.sequence + 1;
        book.add_level(OrderType::Bid, 99.1, 99.1, sequence);
        sequence = book.sequence + 1;
        book.add_level(OrderType::Ask, 99.9, 99.9, sequence);

        let (first_ask, first_bid) = get_first_ask_and_bid(&book);

        assert_eq!(first_ask.0, 99.9);
        assert_eq!(first_ask.1, 99.9);
        assert_eq!(first_bid.0, 99.1);
        assert_eq!(first_bid.1, 99.1);

        // Update BID LEVEL
        sequence = book.sequence + 1;
        book.add_level(OrderType::Bid, 99.1, 99.2, sequence);
        sequence = book.sequence + 1;
        book.add_level(OrderType::Ask, 99.9, 99.8, sequence);

        let (first_ask, first_bid) = get_first_ask_and_bid(&book);

        assert_eq!(first_ask.0, 99.9); //price
        assert_eq!(first_ask.1, 99.8); //size
        assert_eq!(first_bid.0, 99.1); //price
        assert_eq!(first_bid.1, 99.2); //size

        // Remove LEVEL
        sequence = book.sequence + 1;
        book.remove_level(OrderType::Bid, 99.1, sequence);
        sequence = book.sequence + 1;
        book.remove_level(OrderType::Ask, 99.9, sequence);

        let (first_ask, first_bid) = get_first_ask_and_bid(&book);

        assert_eq!(first_ask.0, 100.0);
        assert_eq!(first_ask.1, 100.0);
        assert_eq!(first_bid.0, 99.0);
        assert_eq!(first_bid.1, 99.0);
    }

    #[test]
    fn test_create_snapshot() {
        let bytes = std::fs::read("snapshots/Binance:BTC_USDT").unwrap();
        let book: OrderBook = bytes.try_into().unwrap();
        assert_eq!(book.instrument, "Binance:BTC/USDT");
    }

    #[test]
    fn get_snapshot() {
        let bytes = std::fs::read("snapshots/Binance:BTC_USDT").unwrap();
        let mut book: OrderBook = bytes.clone().try_into().unwrap();
        let snapshot: SnapshotMessage = book.clone().into();

        let snapshot: SnapshotMessage = book.clone().into();
        let mut buf: Vec<u8> = Vec::new();
        snapshot.encode(&mut buf);

        book = buf.clone().try_into().unwrap();

        // print_book(book.clone(), 50 as usize);

        let bid_level = book
            .bids
            .iter()
            .rev()
            .take(1)
            .map(|(a, b)| b.clone())
            .collect::<Vec<Level>>();
        let ask_level = book
            .asks
            .iter()
            .take(1)
            .map(|(a, b)| b.clone())
            .collect::<Vec<Level>>();
        assert_eq!(ask_level[0].price.to_f64().unwrap(), 9017.78f64);
        assert_eq!(ask_level[0].size.to_f64().unwrap(), 0.170818f64);
        assert_eq!(bid_level[0].price.to_f64().unwrap(), 9015.85f64);
        assert_eq!(bid_level[0].size.to_f64().unwrap(), 0.027722000000000004f64);
    }
}
