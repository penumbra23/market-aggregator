use std::collections::BTreeMap;

use serde::{Serialize, Deserialize};

pub type Decimal = rust_decimal::Decimal;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderbookUpdate {
    pub stream: String,
    pub bids: BTreeMap<Decimal, Decimal>,
    pub asks: BTreeMap<Decimal, Decimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Orderbook {
    
}