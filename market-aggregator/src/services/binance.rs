use std::{task::Poll, pin::Pin};

use futures::{SinkExt, StreamExt, Stream};
use pin_project::pin_project;
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, tungstenite::Message};

use super::{Result, OrderBookStream, OrderbookUpdate, Decimal};

#[derive(Serialize)]
struct BinanceSubscription {
    method: String,
    params: Vec<String>,
    id: i32,
}

#[derive(Debug, Deserialize)]
pub  struct BinanceOrderBookUpdate {
    asks: Vec<(Decimal, Decimal)>,
    bids: Vec<(Decimal, Decimal)>,
}

/// Async stream for reading Binance orderbook data.
#[pin_project]
pub struct BinanceStream {
    subscription: BinanceSubscription,
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
            return OrderbookUpdate{
                stream: String::from("binance"),
                asks: bel.asks,
                bids: bel.bids
            };
        });
        Ok(Box::new(result_stream))
    }
}