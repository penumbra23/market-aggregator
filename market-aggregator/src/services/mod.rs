pub mod binance;
pub mod bitstamp;

use common::OrderbookUpdate;

use reqwest::StatusCode;

#[derive(Debug)]
pub struct OrderbookError {
    pub details: String
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

type Result<T> = std::result::Result<T, OrderbookError>;

/// Trait for streams that serve `OrderBookUpdate`s.
/// Each new service fetching orderbook entries needs to imlement `OrderbookConnection`.
#[async_trait::async_trait]
pub trait OrderbookConnection: futures::Stream<Item = OrderbookUpdate> {
    async fn fetch_snapshot(&mut self, market_pair: &str) -> Result<OrderbookUpdate>;
    async fn connect(&mut self) -> Result<()>;
}