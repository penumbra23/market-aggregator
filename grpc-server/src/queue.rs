use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc::{Sender, Receiver};

pub struct OrderbookSubscriber {
    // tx: Sender<OrderbookUpd>,
    // rx: Receiver<T>,
}

#[async_trait]
impl amqprs::consumer::AsyncConsumer for OrderbookSubscriber {
  async fn consume(&mut self, _channel: &amqprs::channel::Channel, deliver: amqprs::Deliver, basic_properties: amqprs::BasicProperties, content: Vec<u8>) {
    let exchange_name = deliver.exchange();

    let routing_key = deliver.routing_key();

    let content = Arc::new(content);
    println!("content: {}", std::str::from_utf8(&content).unwrap());
  }
}