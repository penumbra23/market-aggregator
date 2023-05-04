use std::{collections::HashMap, cmp::Ordering};

use amqprs::{connection::{Connection, OpenConnectionArguments}, channel::{ExchangeDeclareArguments, ExchangeType, BasicPublishArguments}, BasicProperties};
use common::Decimal;
use futures::StreamExt;
use merge_streams::MergeStreams;
use services::{binance::{BinanceStream}, bitstamp::BitstampStream};

use crate::services::OrderBookSnapshot;

mod services;

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
    
    let mut binance_client = BinanceStream::new("btcusdt")
        .await
        .unwrap();
    
    let mut bitstamp_client = BitstampStream::new("btcusdt")
        .await
        .unwrap();

    let mut orderbook = HashMap::new();

    let result = binance_client.fetch_snapshot("btcusdt").await.unwrap();
    let result2 = bitstamp_client.fetch_snapshot("btcusdt").await.unwrap();

    orderbook.insert(result.stream.clone(), result);
    orderbook.insert(result2.stream.clone(), result2);

    binance_client.connect().await?;
    bitstamp_client.connect().await?;

    let mut final_stream = (binance_client, bitstamp_client).merge();

    while let Some(order) = final_stream.next().await {
        let stream = order.stream;
        let orderbook_update = match orderbook.get_mut(&stream) {
            Some(nn) => nn,
            None => continue,
        };

        orderbook_update.bids.retain(|_k, &mut v| v.cmp(&Decimal::ZERO) != Ordering::Equal);
        orderbook_update.asks.retain(|_k, &mut v| v.cmp(&Decimal::ZERO) != Ordering::Equal);

        for bid in order.bids {
            orderbook_update.bids.insert(bid.0, bid.1);
        }

        for ask in order.asks {
            orderbook_update.asks.insert(ask.0, ask.1);
        }
        
        let args = BasicPublishArguments::new(exchange_name, format!("rates.{}", stream).as_str());
    
        println!("{:?}", &orderbook);

        let content = serde_json::to_string(&orderbook).unwrap();

        channel
            .basic_publish(
                BasicProperties::default()
                .with_persistence(true)
                .finish(), 
                content.into(),
                args)
                .await?;
        channel.close().await;
        return Ok(());
    }

    Ok(())
}
