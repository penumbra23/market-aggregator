pub mod binance;
pub mod bitstamp;

use futures::Stream;
use serde::{Serialize, Deserialize};

type Decimal = decimal::d128;

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
    fn from(_value: tokio_tungstenite::tungstenite::Error) -> Self {
        todo!()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderbookUpdate {
    stream: String,
    bids: Vec<(Decimal, Decimal)>,
    asks: Vec<(Decimal, Decimal)>,
}

type Result<T> = std::result::Result<T, OrderbookError>;

/// Trait for streams that serve `OrderBookUpdate`s.
/// Each new service fetching orderbook entries needs to imlement `OrderBookStream`.
#[async_trait::async_trait]
pub trait OrderBookStream: Stream {
    async fn get_ob_stream(self) -> Result<Box<dyn Stream<Item = OrderbookUpdate> + Unpin>>;
}