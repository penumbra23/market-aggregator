use async_trait::async_trait;
use common::{OrderbookQueueItem, Decimal};
use tokio::sync::mpsc::{Sender, Receiver};

use crate::types::Orderbook;

pub struct OrderbookSubscriber {
    // tx: Sender<OrderbookUpd>,
    // rx: Receiver<T>,
    orderbook: Orderbook
}

impl OrderbookSubscriber {
  pub fn new() -> Self {
    Self {
      orderbook: Orderbook::new(10)
    }
  }
}

#[async_trait]
impl amqprs::consumer::AsyncConsumer for OrderbookSubscriber {
  async fn consume(&mut self, _channel: &amqprs::channel::Channel, deliver: amqprs::Deliver, basic_properties: amqprs::BasicProperties, content: Vec<u8>) {
    let exchange_name = deliver.exchange();

    let routing_key = deliver.routing_key();

    // TODO: handle error
    let orderbook_item: OrderbookQueueItem = serde_json::from_slice(&content).unwrap();
    println!("content: {:?}", orderbook_item);
  }
}