use std::{collections::BTreeMap, cmp::Ordering};

use serde::{Serialize, Deserialize};

pub type Decimal = rust_decimal::Decimal;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderbookUpdate {
    pub stream: String,
    pub bids: BTreeMap<Decimal, Decimal>,
    pub asks: BTreeMap<Decimal, Decimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderbookQueueItem {
    pub stream: String,
    pub asks: Vec<(Decimal, Decimal)>,
    pub bids: Vec<(Decimal, Decimal)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Orderbook {
    pub stream: String,
    pub bids: BTreeMap<Decimal, Decimal>,
    pub asks: BTreeMap<Decimal, Decimal>,
}

impl Into<OrderbookQueueItem> for Orderbook {
    fn into(self) -> OrderbookQueueItem {
        let mut bids: Vec<(Decimal, Decimal)> = self.bids.into_iter().collect();
        bids.sort_by(|a, b| b.cmp(a));
        let asks = self.asks.into_iter().collect();

        OrderbookQueueItem {
            stream: self.stream,
            asks,
            bids,
        }
    }
}

impl Orderbook {
    pub fn new(stream: &str) -> Self {
        Self {
            stream: stream.to_string(),
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
        }
    }

    pub fn update_book(&mut self, order_update: &OrderbookUpdate) {
        for bid in order_update.bids.iter() {
            self.bids.insert(*bid.0, *bid.1);
        }
        
        for ask in order_update.asks.iter() {
            self.asks.insert(*ask.0, *ask.1);
        }

        self.bids.retain(|_k, &mut v| v.cmp(&Decimal::ZERO) != Ordering::Equal);
        self.asks.retain(|_k, &mut v| v.cmp(&Decimal::ZERO) != Ordering::Equal);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{Orderbook, OrderbookUpdate, Decimal, OrderbookQueueItem};

    fn is_sorted<T, F>(data: &[T], f: F) -> bool
    where
        T: Ord,
        F: Fn(&T, &T) -> bool
    {
        data.windows(2).all(|a| f(&a[0], &a[1]))
    }

    fn create_asks_bids() -> (BTreeMap<Decimal, Decimal>, BTreeMap<Decimal, Decimal>) {
        let mut bids: BTreeMap<Decimal, Decimal> = BTreeMap::new();
        bids.insert(Decimal::from(5), Decimal::from(10));
        bids.insert(Decimal::from(1), Decimal::from(12));
        bids.insert(Decimal::from(2), Decimal::from(1));

        let mut asks: BTreeMap<Decimal, Decimal> = BTreeMap::new();
        asks.insert(Decimal::from(7), Decimal::from(12));
        asks.insert(Decimal::from(1), Decimal::from(1));
        asks.insert(Decimal::from(3), Decimal::from(1));

        (asks, bids)
    }

    #[test]
    fn orderbook_update() {
        let mut orderbook = Orderbook::new("binance");

        let (mut asks, mut bids) = create_asks_bids();

        orderbook.update_book(&OrderbookUpdate{
            stream: "binance".to_owned(),
            asks: asks.clone(),
            bids: bids.clone(),
        });

        assert_eq!(orderbook.bids.len(), 3);
        assert_eq!(orderbook.asks.len(), 3);

        asks.clear();
        asks.insert(Decimal::from(7), Decimal::from(0));

        bids.clear();
        bids.insert(Decimal::from(5), Decimal::from(8));
        bids.insert(Decimal::from(1), Decimal::from(0));
        bids.insert(Decimal::from(88), Decimal::from(1));

        orderbook.update_book(&OrderbookUpdate{
            stream: "binance".to_owned(),
            asks,
            bids,
        });

        assert_eq!(orderbook.bids.len(), 3);
        assert_eq!(*orderbook.bids.get(&Decimal::from(5)).unwrap(), Decimal::from(8));
        assert_eq!(orderbook.asks.len(), 2);
    }

    #[test]
    fn orderbook_to_queue_item() {
        let mut orderbook = Orderbook::new("binance");
        let (asks, bids) = create_asks_bids();

        orderbook.update_book(&OrderbookUpdate{
            stream: "binance".to_owned(),
            asks: asks,
            bids: bids,
        });

        let ob_queue_item: OrderbookQueueItem = orderbook.into();

        assert_eq!(ob_queue_item.asks.len(), 3);
        assert_eq!(ob_queue_item.bids.len(), 3);
        assert!(is_sorted(&ob_queue_item.asks, |a, b| a < b));
        assert!(is_sorted(&ob_queue_item.bids, |a, b| a > b));
    }
}