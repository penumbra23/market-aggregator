use std::{pin::Pin, task::Poll, collections::{HashMap, BTreeMap}, sync::{Arc, RwLock}};

use futures::{Stream, SinkExt, StreamExt};
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, tungstenite::Message};

use super::{OrderBookStream, OrderbookUpdate, Result, Decimal, OrderbookError};

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
        let bitstamp_socket_url = "wss://ws.bitstamp.net";
        let (bitstamp_ws_stream, _) = tokio_tungstenite::connect_async(bitstamp_socket_url).await?;
        let bitstamp_subscription = BitstampSubscription {
            event: "bts:subscribe".into(),
            data: [("channel".into(), format!("order_book_{}", market_pair).into())]
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
    type Item = BitstampOrderBookUpdate;

    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let poll = self.project().inner_stream.poll_next(cx);
        match poll {
            Poll::Ready(Some(poll_result)) => {
                match poll_result {
                    Ok(msg) => {
                        if let Message::Text(text) = msg {
                            match serde_json::from_str::<BitstampOrderBookUpdate>(&text) {
                                Ok(order) => {
                                    return Poll::Ready(Some(order));
                                },
                                Err(err) => {
                                    // TODO: log error
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
            }
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            },
        }
    }
}

#[async_trait::async_trait]
impl OrderBookStream for BitstampStream {
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

        let last_update = self.start_time.clone();

        let result_stream = self.map::<OrderbookUpdate, _>(move |bel|{
            // TODO: handle error properly
            let timestamp = bel.data.timestamp.parse::<u64>().unwrap();
            if timestamp <= *last_update.read().unwrap() {
                return OrderbookUpdate {
                    stream: String::from("bitstamp"),
                    asks: BTreeMap::default(),
                    bids: BTreeMap::default(),
                };
            }

            // // Update the last read id
            let mut lu = last_update.write().unwrap();
            *lu = timestamp;

            return OrderbookUpdate {
                stream: String::from("bitstamp"),
                asks: bel.data.asks.into_iter().map(|x| (x.0, x.1)).collect(),
                bids: bel.data.bids.into_iter().map(|x| (x.0, x.1)).collect(),
            };
        });
        Ok(Box::new(result_stream))
    }
}