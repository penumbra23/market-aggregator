use std::collections::HashMap;

use amqprs::{connection::{Connection, OpenConnectionArguments}, channel::{ExchangeDeclareArguments, ExchangeType, BasicPublishArguments}, BasicProperties};
use common::Orderbook;
use futures::{stream::select_all, StreamExt};
use services::{binance::BinanceStream, bitstamp::BitstampStream, OrderbookError};

use crate::services::OrderbookConnection;

mod services;

struct App {
    clients: HashMap<String, Box<dyn OrderbookConnection + Unpin>>,
    orderbooks: HashMap<String, Orderbook>,
}

impl App {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            orderbooks: HashMap::new(),
        }
    }

    pub fn add_client(&mut self, stream: &str, client: Box<dyn OrderbookConnection + Unpin>) -> Result<(), OrderbookError> {
        if self.clients.contains_key(stream) {
            return Err(OrderbookError{ details: String::from("Client already present") });
        }

        self.clients.insert(stream.to_owned(), client);
        self.orderbooks.insert(stream.to_owned(), Orderbook::new(stream));

        Ok(())
    }

    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (_, cl) in &mut self.clients {
            // TODO: check res
            let res = cl.connect().await;
        }

        let mut streams = self.clients
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<Box<dyn OrderbookConnection + Unpin>>>();

        while let Some(order) = select_all(streams.iter_mut()).next().await {
            println!("O: {:?}", order);
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::open(&OpenConnectionArguments::new(
        "localhost",
        5672,
        "user",
        "password",
    ))
    .await?;

    let channel = connection.open_channel(None).await.unwrap();

    let exchange_name = "orderbook";

    channel.exchange_declare(
            ExchangeDeclareArguments::of_type(exchange_name, ExchangeType::Topic)
            .durable(true)
            .finish()
        ).await?;
    
    let binance_client = BinanceStream::new("btcusdt")
        .await
        .unwrap();
    
    let bitstamp_client = BitstampStream::new("btcusdt")
        .await
        .unwrap();

    let mut app = App::new();

    app.add_client("binance", Box::new(binance_client))?;
    app.add_client("bitstamp", Box::new(bitstamp_client))?;

    app.run().await?;

    Ok(())
}
