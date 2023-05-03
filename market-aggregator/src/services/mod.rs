pub mod binance;
pub mod bitstamp;

use std::collections::BTreeMap;

use futures::Stream;
use reqwest::StatusCode;
use serde::{Serialize, Deserialize};

type Decimal = rust_decimal::Decimal;

#[derive(Debug)]
pub struct OrderbookError {
    details: String
}

impl std::fmt::Display for OrderbookError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,"{}", self.details)
    }
}

impl std::error::Error for OrderbookError{}

impl From<tokio_tungstenite::tungstenite::Error> for OrderbookError {
    fn from(value: tokio_tungstenite::tungstenite::Error) -> Self {
        Self {
            details: value.to_string(),
        }
    }
}

impl From<reqwest::Error> for OrderbookError {
    fn from(value: reqwest::Error) -> Self {
        Self {
            details: format!("code: {}; details: {}", value.status().unwrap_or(StatusCode::default()), value.to_string())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderbookUpdate {
    stream: String,
    bids: BTreeMap<Decimal, Decimal>,
    asks: BTreeMap<Decimal, Decimal>,
}

type Result<T> = std::result::Result<T, OrderbookError>;

/// Trait for streams that serve `OrderBookUpdate`s.
/// Each new service fetching orderbook entries needs to imlement `OrderBookStream`.
#[async_trait::async_trait]
pub trait OrderBookStream: Stream {
    async fn fetch_snapshot(&mut self, market_pair: &str) -> Result<OrderbookUpdate>;
    async fn get_ob_stream(self) -> Result<Box<dyn Stream<Item = OrderbookUpdate> + Unpin>>;
}