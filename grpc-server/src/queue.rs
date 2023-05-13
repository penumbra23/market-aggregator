use async_trait::async_trait;
use common::{OrderbookQueueItem};
use tokio::sync::broadcast::Sender;
use tonic::Status;

use crate::types::{Orderbook, OrderbookEntry};
use crate::server::orderbook::Summary;

pub struct OrderbookSubscriber {
    tx: Sender<Result<Summary, Status>>,
    orderbook: Orderbook,
}

impl OrderbookSubscriber {
  pub fn new(tx: Sender<Result<Summary, Status>>) -> Self {
    Self {
      orderbook: Orderbook::new(10),
      tx,
    }
  }
}


#[async_trait]
impl amqprs::consumer::AsyncConsumer for OrderbookSubscriber {
  async fn consume(&mut self, _channel: &amqprs::channel::Channel, _deliver: amqprs::Deliver, _basic_properties: amqprs::BasicProperties, content: Vec<u8>) {
    // TODO: handle error
    let orderbook_item: OrderbookQueueItem = serde_json::from_slice(&content).unwrap();

    for ask in &orderbook_item.asks {
      self.orderbook.update_asks(OrderbookEntry{
        exchange: orderbook_item.stream.clone(),
        price: ask.0,
        amount: ask.1,
      });
    }

    for bid in &orderbook_item.bids {
      self.orderbook.update_bids(OrderbookEntry{
        exchange: orderbook_item.stream.clone(),
        price: bid.0,
        amount: bid.1,
      });
    }

    // TODO: handle error
    self.tx.send(Ok(self.orderbook.clone().into())).unwrap();
  }
}