use std::{task::Poll, pin::Pin, collections::BTreeMap, sync::{Arc, RwLock}};

use futures::{SinkExt, StreamExt, Stream};
use pin_project::pin_project;
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, tungstenite::Message};

use super::{Result, OrderBookStream, OrderbookUpdate, Decimal, OrderbookError};

#[derive(Serialize)]
struct BinanceSubscription {
    method: String,
    params: Vec<String>,
    id: i32,
}

#[derive(Deserialize)]
pub struct BinanceOrderBookSnapshot {
    #[serde(rename = "lastUpdateId")]
    last_update_id: u64,
    asks: Vec<(Decimal, Decimal)>,
    bids: Vec<(Decimal, Decimal)>,
}

impl Into<OrderbookUpdate> for BinanceOrderBookSnapshot {
    fn into(self) -> OrderbookUpdate {
        OrderbookUpdate {
            stream: String::from("binance"),
            bids: self.bids.into_iter().map(|x| (x.0, x.1)).collect(),
            asks: self.asks.into_iter().map(|x| (x.0, x.1)).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub  struct BinanceOrderBookUpdate {
    #[serde(rename = "a")]
    asks: Vec<(Decimal, Decimal)>,
    #[serde(rename = "b")]
    bids: Vec<(Decimal, Decimal)>,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    last_update_id: u64
}

/// Async stream for reading Binance orderbook data.
#[pin_project]
pub struct BinanceStream {
    subscription: BinanceSubscription,
    last_update: Arc<RwLock<u64>>,
    #[pin]
    inner_stream: WebSocketStream<MaybeTlsStream<TcpStream>>
}

impl BinanceStream {
    pub async fn new(market_pair: &str, depth: u8, freq: u16) -> Result<BinanceStream> {
        let sub = BinanceSubscription {
            method: "SUBSCRIBE".into(),
            params: vec![format!("{}@depth", market_pair).into()],
            id: 1,
        };

        let binance_socket_url = "wss://stream.binance.com:9443/ws";
        let  (binance_ws_stream, _) = tokio_tungstenite::connect_async(binance_socket_url).await?;

        Ok(Self {
            subscription: sub,
            inner_stream: binance_ws_stream,
            last_update: Arc::new(RwLock::new(0)),
        })
    }
}

impl Stream for BinanceStream {
    type Item = BinanceOrderBookUpdate;

    /// Wrapped `poll_next` for converting the data. In case the conversion fails, it keeps polling the stream.
    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let poll = self.project().inner_stream.poll_next(cx);
        match poll {
            Poll::Ready(Some(poll_result)) => {
                match poll_result {
                    Ok(msg) => {
                        if let Message::Text(text) = msg {
                            match serde_json::from_str::<BinanceOrderBookUpdate>(&text) {
                                Ok(order) => {
                                    return Poll::Ready(Some(order));
                                },
                                Err(err) => {
                                    // TODO: log
                                    println!("Err {} {}", err, text);
                                },
                            };
                        }
                    },
                    Err(err) => {
                        // TODO: log error
                        println!("Err {}", err);
                    },
                };
                
                cx.waker().wake_by_ref();
                return Poll::Pending;
            },
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            },
        }
    }
}


#[async_trait::async_trait]
impl OrderBookStream for BinanceStream {
    async fn fetch_snapshot(&mut self, market_pair: &str) -> Result<OrderbookUpdate> {
        let url = format!("https://api.binance.com/api/v3/depth?symbol={}&limit=10", market_pair.to_ascii_uppercase());
        let response = reqwest::get(&url).await?.text().await?;
        let orderbook: BinanceOrderBookSnapshot = match serde_json::from_str(&response) {
            Ok(ob) => ob,
            Err(err) => return Err(OrderbookError{details: err.to_string()}),
        };

        // Set orderbook timestamp
        self.last_update = Arc::new(RwLock::new(orderbook.last_update_id));

        Ok(orderbook.into())
    }

    async fn get_ob_stream(mut self) -> Result<Box<dyn Stream<Item = OrderbookUpdate> + Unpin>> {
        self.inner_stream
            .send(Message::Text(json!(self.subscription).to_string()))
            .await?;

        match self.inner_stream.next().await {
            Some(msg) => {
                // TODO: check if result is correct
                msg?;
            },
            None => return Err(super::OrderbookError { details: String::from("No response") }),
        }

        let last_update = self.last_update.clone();
        
        let result_stream = self.map::<OrderbookUpdate, _>(move |bel|{
            // First check if the updates are after the last one
            if bel.last_update_id <= *last_update.read().unwrap() {
                return OrderbookUpdate {
                    stream: String::from("binance"),
                    asks: BTreeMap::default(),
                    bids: BTreeMap::default(),
                };
            }

            // Update the last read id
            let mut lu = last_update.write().unwrap();
            *lu = bel.last_update_id;

            return OrderbookUpdate{
                stream: String::from("binance"),
                asks: bel.asks.into_iter().map(|x| (x.0, x.1)).collect(),
                bids: bel.bids.into_iter().map(|x| (x.0, x.1)).collect(),
            };
        });
        Ok(Box::new(result_stream))
    }
}