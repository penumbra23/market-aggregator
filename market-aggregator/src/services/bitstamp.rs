use std::{pin::Pin, task::Poll, collections::HashMap};

use futures::{Stream, SinkExt, StreamExt};
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, tungstenite::Message};

use super::{OrderBookStream, OrderbookUpdate, Result, Decimal};

#[derive(Debug, Deserialize)]
pub struct BitstampOrderBookUpdate {
    data: BitstampOrderBookData
}

#[derive(Debug, Deserialize)]
pub struct BitstampOrderBookData {
    bids: Vec<(Decimal, Decimal)>,
    asks: Vec<(Decimal, Decimal)>,
}

#[derive(Serialize)]
struct BitstampSubscription {
    event: String,
    data: HashMap<String, String>,
}

/// Async stream for reading Bitstamp orderbook data.
#[pin_project]
pub struct BitstampStream {
    subscription: BitstampSubscription,
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

        let result_stream = self.map::<OrderbookUpdate, _>(|bel|{
            return OrderbookUpdate {
                stream: String::from("bitstamp"),
                asks: bel.data.asks,
                bids: bel.data.bids
            };
        });
        Ok(Box::new(result_stream))
    }
}