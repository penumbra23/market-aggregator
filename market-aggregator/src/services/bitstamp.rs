use std::{pin::Pin, task::Poll, collections::{HashMap}, sync::{Arc, RwLock}};

use common::Decimal;
use futures::{Stream, SinkExt, StreamExt, executor::block_on};
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, tungstenite::Message};

use super::{OrderbookConnection, OrderbookUpdate, Result, OrderbookError};

const BITSTAMP_WS_URL: &str = "wss://ws.bitstamp.net";

#[derive(Debug, Deserialize)]
pub struct BitstampOrderBookUpdate {
    data: BitstampOrderBookData
}

#[derive(Debug, Deserialize)]
pub struct BitstampOrderBookData {
    timestamp: String,
    microtimestamp: String,
    bids: Vec<(Decimal, Decimal)>,
    asks: Vec<(Decimal, Decimal)>,
}

#[derive(Serialize)]
struct BitstampSubscription {
    event: String,
    data: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct BitstampOrderBookSnapshot {
    timestamp: String,
    microtimestamp: String,
    asks: Vec<(Decimal, Decimal)>,
    bids: Vec<(Decimal, Decimal)>,
}

impl Into<OrderbookUpdate> for BitstampOrderBookSnapshot {
    fn into(self) -> OrderbookUpdate {
        OrderbookUpdate {
            stream: String::from("bitstamp"),
            bids: self.bids.into_iter().map(|x| (x.0, x.1)).collect(),
            asks: self.asks.into_iter().map(|x| (x.0, x.1)).collect(),
        }
    }
}

/// Async stream for reading Bitstamp orderbook data.
#[pin_project]
pub struct BitstampStream {
    subscription: BitstampSubscription,
    start_time: Arc<RwLock<u64>>,
    #[pin]
    inner_stream: WebSocketStream<MaybeTlsStream<TcpStream>>
}

impl BitstampStream {
    pub async fn new(market_pair: &str) -> Result<BitstampStream> {
        let (bitstamp_ws_stream, _) = tokio_tungstenite::connect_async(
            BITSTAMP_WS_URL
        ).await?;

        let bitstamp_subscription = BitstampSubscription {
            event: "bts:subscribe".into(),
            data: [("channel".into(), format!("diff_order_book_{}", market_pair).into())]
                .into_iter()
                .collect(),
        };

        Ok(Self {
            inner_stream: bitstamp_ws_stream,
            subscription: bitstamp_subscription,
            start_time: Arc::new(RwLock::new(0)),
        })
    }
}

impl Stream for BitstampStream {
    type Item = OrderbookUpdate;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner_stream).poll_next(cx) {
            Poll::Ready(Some(poll_result)) => {
                match poll_result {
                    Ok(msg) => {
                        match msg {
                            Message::Text(text) => {
                                match serde_json::from_str::<BitstampOrderBookUpdate>(&text) {
                                    Ok(order) => {
                                        let last_update = self.start_time.clone();
                                        let timestamp = order.data.timestamp.parse::<u64>().unwrap();
                                        if timestamp <= *last_update.read().unwrap() {
                                            cx.waker().wake_by_ref();
                                            return Poll::Pending;
                                        }

                                        // // Update the last read id
                                        let mut lu = last_update.write().unwrap();
                                        *lu = timestamp;
                                        
                                        return Poll::Ready(Some(OrderbookUpdate {
                                            stream: String::from("bitstamp"),
                                            asks: order.data.asks.into_iter().map(|x| (x.0, x.1)).collect(),
                                            bids: order.data.bids.into_iter().map(|x| (x.0, x.1)).collect(),
                                        }));
                                    },
                                    Err(err) => {
                                        // TODO: log error
                                        println!("Err {} {}", err, text);
                                    },
                                };
                            },
                            Message::Ping(_) => {
                                // TODO: implement pong
                            },
                            Message::Pong(_) => todo!(),
                            Message::Close(_) => {
                                // TODO: check res and retry
                                let _res = block_on(self.connect());
                            },
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
            }
            Poll::Ready(None) => {
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
impl OrderbookConnection for BitstampStream {
    async fn fetch_snapshot(&mut self, market_pair: &str) -> Result<OrderbookUpdate> {
        let url = format!("https://www.bitstamp.net/api/v2/order_book/{}", market_pair.to_ascii_lowercase());
        let response = reqwest::get(url).await?.text().await?;

        let orderbook: BitstampOrderBookSnapshot = match serde_json::from_str(&response) {
          Ok(ob) => ob,
          Err(err) => return Err(OrderbookError{details: err.to_string()})  
        };

        // Set the timestamp of the orderbook
        self.start_time = Arc::new(RwLock::new(orderbook.timestamp.parse::<u64>().unwrap()));

        Ok(orderbook.into())
    }

    async fn connect(&mut self) -> Result<()> {
        let (mut bitstamp_ws_stream, _) = tokio_tungstenite::connect_async(
            BITSTAMP_WS_URL
        ).await?;

        bitstamp_ws_stream
            .send(Message::Text(json!(self.subscription).to_string()))
            .await?;

        match bitstamp_ws_stream.next().await {
            Some(msg) => {
                println!("check msg {}", msg.unwrap());
                // msg?;
            },
            None => return Err(super::OrderbookError { details: String::from("No response") }),
        }
    
        self.inner_stream = bitstamp_ws_stream;

        Ok(())
    }
}