use std::{cmp::{Reverse, Ordering}, collections::BTreeSet};

use common::Decimal;
use sorted_vec::{SortedSet, ReverseSortedSet};

#[derive(Debug)]
pub struct OrderbookEntry {
    pub exchange: String,
    pub price: Decimal,
    pub amount: Decimal,
}

impl PartialEq for OrderbookEntry {
    fn eq(&self, other: &Self) -> bool {
        self.price.eq(&other.price)
    }
}

impl Eq for OrderbookEntry {}

impl PartialOrd for OrderbookEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.price.partial_cmp(&other.price)
    }
}

impl Ord for OrderbookEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.price.cmp(&other.price)
    }
}

#[derive(Debug)]
pub struct Orderbook {
    pub spread: f64,
    pub asks: SortedSet<OrderbookEntry>,
    pub bids: ReverseSortedSet<OrderbookEntry>,
    max_cap: usize,
}

impl Orderbook {
    pub fn new(capacity: usize) -> Self {
        Self {
            spread: 0f64,
            asks: SortedSet::with_capacity(capacity),
            bids: ReverseSortedSet::with_capacity(capacity),
            max_cap: capacity,
        }
    }

    pub fn update_asks(&mut self, entry: OrderbookEntry) -> bool {
        let (_, item) = self.asks.replace(entry);
        if self.asks.len() > self.max_cap {
            self.asks.pop();
        }
        item.is_some()
    }

    pub fn update_bids(&mut self, entry: OrderbookEntry) -> bool {
        let (_, item) = self.bids.replace(Reverse(entry));
        if self.bids.len() > self.max_cap {
            self.bids.pop();
        }
        item.is_none()
    }
}

#[cfg(test)]
mod tests {
    use common::Decimal;

    use super::{Orderbook, OrderbookEntry};

    #[test]
    fn check_updates() {
        let mut orderbook = Orderbook::new(3);
        orderbook.update_asks(OrderbookEntry {
            exchange: "bitstamp".to_string(),
            amount: Decimal::from_f64_retain(0.568).unwrap(),
            price: Decimal::from_f64_retain(24000.0).unwrap(),
        });

        orderbook.update_asks(OrderbookEntry {
            exchange: "binance".to_string(),
            amount: Decimal::from_f64_retain(0.028).unwrap(),
            price: Decimal::from_f64_retain(23000.0).unwrap(),
        });

        orderbook.update_asks(OrderbookEntry {
            exchange: "binance".to_string(),
            amount: Decimal::from_f64_retain(0.008).unwrap(),
            price: Decimal::from_f64_retain(22000.0).unwrap(),
        });

        orderbook.update_asks(OrderbookEntry {
            exchange: "kraken".to_string(),
            amount: Decimal::from_f64_retain(0.108).unwrap(),
            price: Decimal::from_f64_retain(23000.0).unwrap(),
        });

        orderbook.update_asks(OrderbookEntry {
            exchange: "kraken".to_string(),
            amount: Decimal::from_f64_retain(0.108).unwrap(),
            price: Decimal::from_f64_retain(29000.0).unwrap(),
        });

        assert_eq!(orderbook.asks.len(), 3);
        assert_eq!(orderbook.asks[0].amount, Decimal::from_f64_retain(0.008).unwrap());
        assert_eq!(orderbook.asks.last().unwrap().amount, Decimal::from_f64_retain(0.568).unwrap());

        orderbook.update_bids(OrderbookEntry {
            exchange: "kraken".to_string(),
            amount: Decimal::from_f64_retain(0.18).unwrap(),
            price: Decimal::from_f64_retain(22000.0).unwrap(),
        });

        orderbook.update_bids(OrderbookEntry {
            exchange: "kraken".to_string(),
            amount: Decimal::from_f64_retain(0.8).unwrap(),
            price: Decimal::from_f64_retain(24000.0).unwrap(),
        });

        orderbook.update_bids(OrderbookEntry {
            exchange: "kraken".to_string(),
            amount: Decimal::from_f64_retain(0.44).unwrap(),
            price: Decimal::from_f64_retain(25000.0).unwrap(),
        });

        orderbook.update_bids(OrderbookEntry {
            exchange: "kraken".to_string(),
            amount: Decimal::from_f64_retain(0.208).unwrap(),
            price: Decimal::from_f64_retain(24000.0).unwrap(),
        });

        assert_eq!(orderbook.bids.len(), 3);
        assert_eq!(orderbook.bids[0].0.amount, Decimal::from_f64_retain(0.44).unwrap());
        assert_eq!(orderbook.bids.last().unwrap().0.amount, Decimal::from_f64_retain(0.18).unwrap());

    }
}