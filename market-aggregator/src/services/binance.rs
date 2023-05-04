use std::{task::Poll, pin::Pin, sync::{Arc, RwLock}};

use common::Decimal;
use futures::{SinkExt, StreamExt, Stream, executor::block_on};
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, tungstenite::Message};

use super::{Result, OrderbookUpdate, OrderbookError, OrderBookSnapshot};

const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

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
pub struct BinanceStream {
    subscription: BinanceSubscription,
    last_update: Arc<RwLock<u64>>,
    inner_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl BinanceStream {
    pub async fn new(market_pair: &str) -> Result<BinanceStream> {
        let sub = BinanceSubscription {
            method: "SUBSCRIBE".into(),
            params: vec![format!("{}@depth", market_pair).into()],
            id: 1,
        };
        
        let  (binance_ws_stream, _) = tokio_tungstenite::connect_async(
            BINANCE_WS_URL
        ).await?;
        
        Ok(Self {
            subscription: sub,
            inner_stream: binance_ws_stream,
            last_update: Arc::new(RwLock::new(0)),
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        let (mut binance_ws_stream, _) = tokio_tungstenite::connect_async(
            BINANCE_WS_URL
        ).await?;
        
        binance_ws_stream
            .send(Message::Text(json!(self.subscription).to_string()))
            .await?;

        match binance_ws_stream.next().await {
            Some(msg) => {
                println!("check msg {}", msg.unwrap());
                // msg?;
            },
            None => return Err(super::OrderbookError { details: String::from("No response") }),
        }
        
        self.inner_stream = binance_ws_stream;

        Ok(())
    }
}

impl Stream for BinanceStream {
    type Item = OrderbookUpdate;

    /// Wrapped `poll_next` for converting the data. In case the conversion fails, it keeps polling the stream.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner_stream).poll_next(cx) {
            Poll::Ready(Some(poll_result)) => {
                match poll_result {
                    Ok(msg) => {
                        match msg {
                            Message::Text(text) => {
                                match serde_json::from_str::<BinanceOrderBookUpdate>(&text) {
                                    Ok(order) => {
                                        let last_update = self.last_update.clone();
                                        if order.last_update_id <= *last_update.read().unwrap() {
                                            cx.waker().wake_by_ref();
                                            return Poll::Pending;
                                        }
                            
                                        // Update the last read id
                                        let mut lu = self.last_update.write().unwrap();
                                        *lu = order.last_update_id;
                            
                                        return Poll::Ready(Some(
                                            OrderbookUpdate{
                                                stream: String::from("binance"),
                                                asks: order.asks.into_iter().map(|x| (x.0, x.1)).collect(),
                                                bids: order.bids.into_iter().map(|x| (x.0, x.1)).collect(),
                                            }));
                                    },
                                    Err(err) => {
                                        // TODO: log
                                        println!("Err {} {}", err, text);
                                    },
                                };
                            },
                            Message::Close(_) => {
                                // TODO: check res and retry
                                let _res = block_on(self.connect());
                            },
                            Message::Ping(p) => println!("ping {:?}", p),
                            Message::Pong(p) => println!("pong {:?}", p),
                            _ => {},

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
            Poll::Ready(None) => {
                // TODO: check res and retry
                let _res = block_on(self.connect());
                cx.waker().wake_by_ref();
                return Poll::Pending;
            },
            Poll::Pending => {
                return Poll::Pending;
            },
        }
    }
}


#[async_trait::async_trait]
impl OrderBookSnapshot for BinanceStream {
    async fn fetch_snapshot(&mut self, market_pair: &str) -> Result<OrderbookUpdate> {
        let url = format!("https://api.binance.com/api/v3/depth?symbol={}&limit=50", market_pair.to_ascii_uppercase());
        let response = reqwest::get(&url).await?.text().await?;
        let orderbook: BinanceOrderBookSnapshot = match serde_json::from_str(&response) {
            Ok(ob) => ob,
            Err(err) => return Err(OrderbookError{details: err.to_string()}),
        };

        // Set orderbook timestamp
        self.last_update = Arc::new(RwLock::new(orderbook.last_update_id));

        Ok(orderbook.into())
    }
}