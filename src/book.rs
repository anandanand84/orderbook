extern crate stock_messages;
extern crate serde;
pub mod book {
    use stock_messages::stock_messages::{Side, SnapshotMessage, PriceLevel, BookInfo, Type, LevelUpdate};
    use std::collections::BTreeMap;
    use std::time::SystemTime;
    use prost::Message;
    use num_traits::identities::Zero;
    use std::ops::{Mul, Add, Sub};
    use std::convert::TryInto;
    use std::convert::TryFrom;
    use bigdecimal::BigDecimal;
    use num_traits::cast::ToPrimitive;
    use crate::itertools::Itertools;
    use serde::{Serialize, Deserialize};
    use std::ops::Bound::{ Included };

    
    pub fn group_decimal(price:f64, group_size:f64, group_lower:bool) -> f64{
        let group = (price / group_size) as u64;
        let mut current_price = (group as f64) * group_size;
        if !group_lower { 
            let group_result = price / group_size;
            if group_result > (group as f64) { 
                current_price = current_price + group_size 
            } 
        }
        current_price
    }

    pub type Price = BigDecimal;
    pub type Size = BigDecimal;
    pub type Value = BigDecimal;
    
    #[derive(Debug, Hash, Eq, PartialEq, Clone)]
    pub struct Level {
        pub price : Price,
        pub size : Size,
        pub value : Value
    }

    pub enum OrderType {
        Bid = 1,
        Ask = 2
    }

    impl Level {
        pub fn new(price:f64, size:f64) -> Level{
            Level {
                price: BigDecimal::from(price),
                size: BigDecimal::from(size),
                value: BigDecimal::from(price).mul(BigDecimal::from(size)),
            }
        }
    }

    impl From<PriceLevel> for Level {
        fn from(level:PriceLevel) -> Self {
            Level::new(level.price, level.total_size)
        }
    }

    impl Into<PriceLevel> for Level {
        fn into(self) -> PriceLevel{
            PriceLevel {
                price: self.price.clone().to_string().parse().unwrap_or(0f64),
                total_size: self.size.clone().to_string().parse().unwrap_or(0f64),
                total_value: BigDecimal::from(self.price).mul(BigDecimal::from(self.size)).clone().to_string().parse().unwrap_or(0f64),
            }
        }
    }
    
    impl Into<SnapshotLevel> for Level {
        fn into(self) -> SnapshotLevel{
            SnapshotLevel {
                price: self.price.clone().to_string().parse().unwrap_or(0f64),
                total_size: self.size.clone().to_string().parse().unwrap_or(0f64),
                total_value: BigDecimal::from(self.price).mul(BigDecimal::from(self.size)).clone().to_string().parse().unwrap_or(0f64),
                relative_size : 0
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct OrderBook {
        pub instrument : String,
        pub sequence: u64,
        pub bids : BTreeMap<Price, Level>,
        pub asks : BTreeMap<Price, Level>,
        
        pub bids_total: Size,
        pub bids_value_total: Value,
        pub asks_total: Size,
        pub asks_value_total: Value,
        
        price_precision: u8,
        size_precision: u8,
        bids_grouped: BTreeMap<Price, Level>,
        asks_grouped: BTreeMap<Price, Level>,
        group: Option<f64>,
        // orderPool: OrderPool = {};
    }


    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct SnapshotLevel {
        pub relative_size : i64,
        pub price : f64,
        pub total_size : f64,
        pub total_value : f64
    }

    impl SnapshotLevel {
        pub fn new() -> SnapshotLevel {
            SnapshotLevel {
                price : 0.0,
                total_size : 0.0,
                total_value : 0.0,
                relative_size : 0
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct OrderBookInfo {
        pub asks_total : f64,
        pub asks_value_total : f64,
        pub bids_value_total : f64,
        pub bids_total : f64,
        pub spread : String,
        pub sequence: u64
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct OrderBookSnapshot {
        pub instrument : String, //"Bittrex:ETH/USDT"
        pub time :u64,  //"2017-10-14T20:08:50.920Z"
        pub info : OrderBookInfo,
        pub asks : Vec<SnapshotLevel>,
        pub bids : Vec<SnapshotLevel>,
        pub cum_bid_values : Vec<SnapshotLevel>,
        pub cum_ask_values : Vec<SnapshotLevel>
    }

    impl From<SnapshotMessage> for OrderBook {
        fn from(snapshot: SnapshotMessage) -> Self {
           let sequence:u64 = snapshot.source_sequence.try_into().unwrap_or(0u64);
           let mut book = OrderBook::new(&snapshot.product_id, sequence);
           book.bids_total = snapshot.bids.iter().fold(BigDecimal::zero(), |total, current| {
               total.add(BigDecimal::from(current.price))
           });
           book.asks_total = snapshot.asks.iter().fold(BigDecimal::zero(), |total, current| {
               total.add(BigDecimal::from(current.price))
           });
           book.bids_value_total = snapshot.bids.iter().fold(BigDecimal::zero(), |total, current| {
               total.add(BigDecimal::from(current.price).mul(BigDecimal::from(current.total_size)))
           });
           book.asks_value_total = snapshot.asks.iter().fold(BigDecimal::zero(), |total, current| {
               total.add(BigDecimal::from(current.price).mul(BigDecimal::from(current.total_size)))
           });
           book.asks = snapshot.asks.into_iter().map(|pricelevel| (BigDecimal::from(pricelevel.price), Level::from(pricelevel))).collect();
           book.bids = snapshot.bids.into_iter().map(|pricelevel| (BigDecimal::from(pricelevel.price), Level::from(pricelevel))).collect();
           book
        }
    }
    
    impl TryFrom<Vec<u8>> for OrderBook {
        type Error = &'static str;
        fn try_from(buf: Vec<u8>) -> Result<Self, Self::Error> {
            let snapshot_decode = SnapshotMessage::decode(buf);
            match snapshot_decode {
                Ok(snapshot) => {
                    Ok(OrderBook::from(snapshot))
                },
                Err(_) => Err("Failed to decode the snapshot")
            }
        }
    }

    impl Into<Vec<u8>> for &OrderBook {
        fn into(self) -> Vec<u8> {
            let info = BookInfo {
                sequence : self.sequence as u32,
                ask_total_size : self.asks_total.to_string().parse().unwrap_or(0f64),
                ask_total_value : self.asks_value_total.to_string().parse().unwrap_or(0f64),
                bid_total_size : self.bids_total.to_string().parse().unwrap_or(0f64),
                bid_tota_value : self.bids_value_total.to_string().parse().unwrap_or(0f64),
            };
            let message = SnapshotMessage {
                trades : vec![],
                r#type : Type::Snapshot.into(),
                exchange : -1,
                info: info,
                product_id : String::from(&self.instrument),
                bids : self.bids.iter()
                        .map(|(_, level)| { level.clone() })
                        .map(|y|  y.clone().into() ).collect(),
                asks : self.asks.iter()
                        .map(|(_, level)| { level.clone() })
                        .map(|y|  y.clone().into() ).collect(),
                source_sequence : self.sequence as i32,
                takers : vec![],
                time : SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64
            };
            let mut buf:Vec<u8> = Vec::new();
            message.encode(&mut buf).unwrap();
            buf
        }
    }

    impl Into<SnapshotMessage> for OrderBook {
        fn into(self: OrderBook) -> SnapshotMessage {
            let info = BookInfo {
                sequence : self.sequence as u32,
                ask_total_size : self.asks_total.to_string().parse().unwrap_or(0f64),
                ask_total_value : self.asks_value_total.to_string().parse().unwrap_or(0f64),
                bid_total_size : self.bids_total.to_string().parse().unwrap_or(0f64),
                bid_tota_value : self.bids_value_total.to_string().parse().unwrap_or(0f64),
            };
            let message = SnapshotMessage {
                trades : vec![],
                r#type : Type::Snapshot.into(),
                exchange : -1,
                info: info,
                product_id : String::from(&self.instrument),
                bids : self.bids.iter()
                        .map(|(_, level)| { level.clone() })
                        .map(|y|  y.clone().into() ).collect(),
                asks : self.asks.iter()
                        .map(|(_, level)| { level.clone() })
                        .map(|y|  y.clone().into() ).collect(),
                source_sequence : self.sequence as i32,
                takers : vec![],
                time : 100u64
            };
            message
        }
    }

    impl OrderBook {
        pub fn new(instrument : &str, sequence: u64) -> OrderBook {
            OrderBook {
                sequence : sequence,
                instrument : String::from(instrument),
                bids : BTreeMap::new(),
                asks : BTreeMap::new(),
                bids_total: BigDecimal::zero(),
                bids_value_total: BigDecimal::zero(),
                asks_total: BigDecimal::zero(),
                asks_value_total: BigDecimal::zero(),
                price_precision: 8,
                size_precision: 8
            }
        }

        pub fn verify_sequence(&self, sequence: i32) -> (bool, bool) {
            let next_sequence = (self.sequence + 1) as i32;
            let received_sequence = sequence;
            if received_sequence < next_sequence {
                println!("old sequenece for {} ignoring current received sequence: {} book sequence: {}", self.instrument, received_sequence, next_sequence);
                return (true, true);
            } else if received_sequence > next_sequence {
                println!("SEQUENCE MISMATCH {} received {}, next {}", self.instrument, received_sequence, next_sequence);
                return (true, false);
            }
            return (false, false);
        }

        pub fn update_level_message(&mut self, level_message: LevelUpdate) -> bool {
            let (stop, valid) = self.verify_sequence(level_message.sequence);
            if stop {
                return valid;
            }
            if level_message.size == 0.0 {
                self.remove_level(OrderType::Bid, level_message.price, level_message.sequence as u64);
                self.remove_level(OrderType::Ask, level_message.price, level_message.sequence as u64);
                return true
            }
            let side = Side::from_i32(level_message.side).unwrap();
            match  side {
                Side::Buy => {
                    self.add_level(OrderType::Bid, level_message.price, level_message.size, level_message.sequence as u64);
                },
                Side::Sell => {
                    self.add_level(OrderType::Ask, level_message.price, level_message.size, level_message.sequence as u64);
                }
            }
            true
        }
        
        pub fn update_level(&mut self, bytes: Vec<u8>) -> bool{
            let level_message:LevelUpdate = LevelUpdate::decode(bytes).unwrap();
            return self.update_level_message(level_message);
        }
        
        pub fn add_level(&mut self, order_type: OrderType, price:f64, size:f64, sequence:u64) -> bool {
            self.sequence = sequence;
            match order_type {
                OrderType::Bid => {
                    self.bids_total = self.bids_total.clone().add(BigDecimal::from(size));
                    self.bids_value_total = self.bids_value_total.clone()
                                        .add(BigDecimal::from(size).mul(BigDecimal::from(price)));
                    self.bids
                        .insert(BigDecimal::from(price), Level::new(price, size));
                },
                OrderType::Ask => {
                    self.asks_total = self.asks_total.clone().add(BigDecimal::from(size));
                    self.asks_value_total = self.asks_value_total.clone()
                                        .add(BigDecimal::from(size).mul(BigDecimal::from(price)));
                    self.asks
                        .insert(BigDecimal::from(price), Level::new(price, size));
                }
            }
            return true
        }
        
        pub fn remove_level(&mut self, order_type: OrderType, price:f64, sequence:u64) -> bool{
            self.sequence = sequence;
            match order_type {
                OrderType::Bid => {
                    let removed_level = self.bids.remove(&BigDecimal::from(price));
                    if let Some(level) = removed_level {
                        self.bids_total = self.bids_total.clone().sub(level.size);
                        self.bids_value_total = self.bids_value_total
                                                    .clone()
                                                    .sub(level.value);
                    }
                },
                OrderType::Ask => {
                    let removed_level = self.asks.remove(&BigDecimal::from(price));
                    if let Some(level) = removed_level {
                        self.asks_total = self.asks_total.clone().sub(level.size);
                        self.asks_value_total = self.asks_value_total.clone()
                                        .sub(level.value);
                    }
                }
            }
            return true;
        }
        pub fn get_levels(&self, count: i32) ->  (Vec<Level>,Vec<Level>) {
            let asks = self.asks.iter().take(count as usize)
            .map(|(x,y)|{ (x.clone(), y.clone())})
            .collect::<BTreeMap<Price, Level>>()
            .iter().rev()
            .map(|(_, level)| { level.clone() })
            .collect::<Vec<Level>>();

            let bids = self.bids.iter()
            .rev().take(count as usize)
            .map(|(_, level)| { level.clone() })
            .collect::<Vec<Level>>();

            (bids, asks)
        }

        pub fn get_spread_percent(&self) -> f64 {
            let bid:f64 = self.get_best_bid();
            let ask:f64 = self.get_best_ask();
            return ((ask - bid) / bid) * 100.0 ;
        }

        pub fn get_spread(&self) -> f64{
            let bid:f64 = self.get_best_bid();
            let ask:f64 = self.get_best_ask();
            return ask - bid;
        }

        pub fn get_best_bid(&self) -> f64 {
            self.bids.keys().rev().take(1).map(|bigdec| bigdec.to_f64().unwrap()).sum()
        }
        
        pub fn get_best_ask(&self) -> f64 {
            self.asks.keys().take(1).map(|bigdec| bigdec.to_f64().unwrap()).sum()
        }

        pub fn get_cumulative_value(&self, order_type: OrderType, start_value: f64, end_value: f64) -> Vec<SnapshotLevel>{

            let mut bid_cum_size:BigDecimal = BigDecimal::default();
            let mut bid_cum_value:BigDecimal = BigDecimal::default();
            let mut cum_bid_values = Vec::new();

            match order_type {
                OrderType::Bid => {
                    for(_, level) in self.bids.range((Included(BigDecimal::from(start_value)), Included(BigDecimal::from(end_value)))).rev() {
                        bid_cum_size = bid_cum_size.add(level.size.clone());
                        bid_cum_value = bid_cum_value.add(level.value.clone());
                        cum_bid_values.push(SnapshotLevel {
                            price : level.price.to_string().parse().unwrap_or(0f64),
                            total_size : bid_cum_size.to_f64().unwrap(),
                            total_value : bid_cum_value.to_f64().unwrap(),
                            relative_size : 0
                        });
                    }
                },
                OrderType::Ask => {
                    for(_, level) in self.asks.range((Included(BigDecimal::from(start_value)), Included(BigDecimal::from(end_value)))) {
                        bid_cum_size = bid_cum_size.add(level.size.clone());
                        bid_cum_value = bid_cum_value.add(level.value.clone());
                        cum_bid_values.push(SnapshotLevel {
                            price : level.price.to_string().parse().unwrap_or(0f64),
                            total_size : bid_cum_size.to_f64().unwrap(),
                            total_value : bid_cum_value.to_f64().unwrap(),
                            relative_size : 0
                        });
                    }
                }
            }
            return cum_bid_values;
        }

        pub fn get_grouped_snapshot(&self, group:f64, count: usize, depth_map_percent: usize) -> OrderBookSnapshot {
            let mut asks = self.asks.iter()
            .map(|(_x,y)|{ 
                let snapshot_level:SnapshotLevel = y.clone().into();
                snapshot_level
            })
            .group_by(|level| {
                group_decimal(level.price, group, false)
            })
            .into_iter()
            .map(|(grouped_price, grouped_levels)| {
                let mut level = SnapshotLevel::new();
                level.price = grouped_price;
                grouped_levels.fold(&mut level, |level, current_level| {
                    level.total_size = level.total_size + current_level.total_size;
                    level.total_value = level.total_value + current_level.total_value;
                    level
                });
                level.into()
            })
            .take(count as usize)
            .collect::<Vec<SnapshotLevel>>();

            let mut bids = self.bids.iter().rev()
            .map(|(_x,y)|{ 
                let snapshot_level:SnapshotLevel = y.clone().into();
                snapshot_level
            })
            .group_by(|level| {
                group_decimal(level.price, group, true)
            })
            .into_iter()
            .map(|(grouped_price, grouped_levels)| {
                let mut level = SnapshotLevel::new();
                level.price = grouped_price;
                grouped_levels.fold(&mut level, |level, current_level| {
                    level.total_size = level.total_size + current_level.total_size;
                    level.total_value = level.total_value + current_level.total_value;
                    level
                });
                level.into()
            })
            .take(count as usize)
            .collect::<Vec<SnapshotLevel>>();

            let mut max_value = asks.iter().chain(bids.iter()).map(|level| {
                (level.total_value * 1000000000000.0) as u64
            }).max().unwrap_or(u64::MAX);

            max_value = max_value / 1000000000000;

            asks.iter_mut().for_each(|level| {
                level.relative_size = (((level.total_value / max_value as f64)) * 38.0) as i64;
            });
            
            bids.iter_mut().for_each(|level| {
                level.relative_size = (((level.total_value / max_value as f64)) * 38.0) as i64;
            });

            let mid_value = (self.get_best_ask() + self.get_best_bid()) / 2.0;

            let bid_bound = mid_value * ( 1.0 - (depth_map_percent as f64) / 100.0);
            
            let ask_bound = mid_value * ( 1.0 + (depth_map_percent as f64) / 100.0);

            // for(key, value) in self.asks.range((Included(BigDecimal::from(mid_value)), Included(BigDecimal::from(ask_bound)))) {
            //     println!("&key, &value, {:?} {:?}", key.clone(), value.clone());
            // }

            let spread = self.get_spread_percent();

            OrderBookSnapshot {
                instrument: self.instrument.to_owned(),
                bids : bids,
                time : 0u64,
                asks : asks,
                info : OrderBookInfo {
                    bids_total : self.bids_total.to_f64().unwrap_or(0.0),
                    bids_value_total : self.bids_value_total.to_f64().unwrap_or(0.0),
                    asks_total : self.asks_total.to_f64().unwrap_or(0.0),
                    asks_value_total : self.asks_value_total.to_f64().unwrap_or(0.0),
                    spread : spread.to_string(),
                    sequence : self.sequence
                },
                cum_ask_values : if depth_map_percent==0 { Vec::new() } else { self.get_cumulative_value(OrderType::Ask, mid_value, ask_bound) },
                cum_bid_values : if depth_map_percent==0 { Vec::new() } else { self.get_cumulative_value(OrderType::Bid, bid_bound, mid_value) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::book::{OrderBook, OrderType, Level, group_decimal};
    use num_traits::cast::ToPrimitive;
    use std::convert::TryInto;
    use stock_messages::stock_messages::{ SnapshotMessage};
    use bytes::{ BytesMut};
    use prost::Message;
    use std::error::Error;

    fn create_asks(book: &mut OrderBook) {
        (100..200).into_iter()
                .zip((100..200).into_iter())
                .for_each(|(price, size)| {
                    let sequence = book.sequence + 1;
                    book.add_level(OrderType::Ask, price as f64, size as f64, sequence);
                })
    }

    fn create_bids(book: &mut OrderBook) {
        (1..100).into_iter()
                .zip((1..100).into_iter())
                .for_each(|(price, size)| {
                    let sequence = book.sequence + 1;
                    book.add_level(OrderType::Bid, price as f64, size as f64, sequence);
                })
    }


    fn print_book(book: OrderBook, count: usize) {
        book.get_levels(count as i32).1.iter()
        .chain(book.get_levels(count as i32).0.iter())
        // .map(|level|{level.clone()})
        .enumerate()
        .for_each(|(index, level)| {
            if index == count {
                println!("==============================================");
            }
            println!("{:?} {:?} {:?}", level.price.to_f64().unwrap(), level.size.to_f64().unwrap(), level.value.to_f64().unwrap());
        })
    }
    
    #[test]
    fn test_group_decimal_1 () {
        let result = group_decimal(6702.01, 1.0, false);
        assert_eq!(result, 6703.00);
        
        let result = group_decimal(6702.01, 1.0, true);
        assert_eq!(result, 6702.00);

        let result = group_decimal(6702.01, 0.01, false);
        assert_eq!(result, 6702.01);

        let result = group_decimal(6702.01, 0.01, true);
        assert_eq!(result, 6702.01);


        let cases = vec![
                //value, grouping, result, 
                (4.324210 , 0.5, 4.00, true),
                (4.324210 , 0.05, 4.3, true),
                (4.324210 , 0.005, 4.32, true),
                (4.624210 , 0.5, 4.5, true),
                (4.624210 , 5.0, 0.0, true),
                (4.324210 , 0.5, 4.50, false),
                (4.324210 , 0.05, 4.35, false),
                (4.324210 , 0.005, 4.325, false),
                (4.624210 , 0.5, 5.0, false),
                (4.624210 , 5.0, 5.0, false),
            ];

            for case in &cases {
                let q = group_decimal(case.0, case.1, case.3);
                assert_eq!(q, case.2);
            }
    }

    #[test]
    fn test_create_book() {
        let mut book = OrderBook::new("instrument", 100);
        create_asks(&mut book);
        create_bids(&mut book);
        print_book(book, 50 as usize);
    }


    fn get_first_ask_and_bid(book:&OrderBook) -> ((f64,f64), (f64,f64)) {
        let first_ask = book.asks.values().take(1)
            .map(|level| (level.price.to_f64().unwrap(), level.size.to_f64().unwrap()))
            .collect::<Vec<(f64,f64)>>()[0];
        let first_bid = book.bids.values().rev().take(1)
            .map(|level| (level.price.to_f64().unwrap(), level.size.to_f64().unwrap()))
            .collect::<Vec<(f64,f64)>>()[0];
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
        let book:OrderBook  = bytes.try_into().unwrap();
        assert_eq!(book.instrument, "Binance:BTC/USDT");
    }
    
    #[test]
    fn get_snapshot() {
        let bytes = std::fs::read("snapshots/Binance:BTC_USDT").unwrap();
        let mut book:OrderBook  = bytes.clone().try_into().unwrap();
        let snapshot:SnapshotMessage = book.clone().into();

        let snapshot:SnapshotMessage = book.clone().into();
        let mut buf:Vec<u8> = Vec::new();
        snapshot.encode(&mut buf);

        book = buf.clone().try_into().unwrap();

        print_book(book.clone(), 50 as usize);
        
        let bid_level = book.bids.iter().rev().take(1).map(|(a,b)| b.clone()).collect::<Vec<Level>>();
        let ask_level = book.asks.iter().take(1).map(|(a,b)| b.clone()).collect::<Vec<Level>>();
        assert_eq!(ask_level[0].price.to_f64().unwrap(), 9017.78f64);
        assert_eq!(ask_level[0].size.to_f64().unwrap(), 0.170818f64);
        assert_eq!(bid_level[0].price.to_f64().unwrap(),  9015.85f64);
        assert_eq!(bid_level[0].size.to_f64().unwrap(), 0.027722000000000004f64);
    }

}