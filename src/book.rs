extern crate stock_messages;
pub mod book {
    use stock_messages::stock_messages::{Side, SnapshotMessage, PriceLevel, BookInfo, Type, LevelUpdate};
    use std::collections::BTreeMap;
    use prost::Message;
    use num_traits::identities::Zero;
    use std::ops::{Mul, Add, Sub};
    use std::convert::TryInto;
    use std::convert::TryFrom;
    use bigdecimal::BigDecimal;
    use std::error::Error;
    use num_traits::cast::ToPrimitive;

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
                price: 0.0f64,
                total_size: 0.0f64,
                total_value: 0.0f64,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct OrderBook {
        pub instrument : String,
        pub sequence: u64,
        bids : BTreeMap<Price, Level>,
        asks : BTreeMap<Price, Level>,
        bids_total: Size,
        bids_value_total: Value,
        asks_total: Size,
        asks_value_total: Value,
        // orderPool: OrderPool = {};
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

    impl Into<SnapshotMessage> for OrderBook {
        fn into(self: OrderBook) -> SnapshotMessage {
            let info = BookInfo {
                sequence : self.sequence as u32,
                ask_total_size : self.asks_total.to_f64().unwrap_or(0.0f64),
                ask_total_value : self.asks_value_total.to_f64().unwrap_or(0.0f64),
                bid_total_size : self.bids_total.to_f64().unwrap_or(0.0f64),
                bid_tota_value : self.bids_value_total.to_f64().unwrap_or(0.0f64),
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
            }
        }

        pub fn update_level(&mut self, bytes: Vec<u8>) {
            let level_message:LevelUpdate = LevelUpdate::decode(bytes).unwrap();
            if level_message.size == 0.0 {
                self.remove_level(OrderType::Bid, level_message.price, level_message.sequence as u64);
                self.remove_level(OrderType::Ask, level_message.price, level_message.sequence as u64);
                ()
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
            ()
        }
        
        pub fn add_level(&mut self, order_type: OrderType, price:f64, size:f64, sequence:u64) {
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
        }
        
        pub fn remove_level(&mut self, order_type: OrderType, price:f64, sequence:u64){
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
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::book::{OrderBook, OrderType};
    use num_traits::cast::ToPrimitive;

    fn create_asks(book: &mut OrderBook) {
        (100..200).into_iter()
                .zip((100..200).into_iter())
                .for_each(|(price, size)| {
                    book.add_level(OrderType::Ask, price as f64, size as f64, 11)
                })
    }

    fn create_bids(book: &mut OrderBook) {
        (1..100).into_iter()
                .zip((1..100).into_iter())
                .for_each(|(price, size)| {
                    book.add_level(OrderType::Bid, price as f64, size as f64, 11)
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
    fn test_create_book() {
        let mut book = OrderBook::new("instrument", 100);
        create_asks(&mut book);
        create_bids(&mut book);
        print_book(book, 50 as usize);
        assert_eq!(10, 10);
    }
    
    #[test]
    fn test_create_snapshot() {
        let mut book = OrderBook::new("instrument", 100);
        create_asks(&mut book);
        create_bids(&mut book);
        assert_eq!(10, 10);
    }
}