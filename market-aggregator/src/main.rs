use futures::StreamExt;
use merge_streams::MergeStreams;
use services::{binance::{BinanceStream}, bitstamp::BitstampStream, OrderBookStream};

mod services;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let binance = BinanceStream::new("btcusdt", 10, 100)
        .await
        .unwrap()
        .get_ob_stream()
        .await
        .unwrap();
    let bitstamp = BitstampStream::new("btcusdt")
        .await
        .unwrap()
        .get_ob_stream()
        .await
        .unwrap();

    let mut final_stream = (binance, bitstamp).merge();

    while let Some(order) = final_stream.next().await {
        println!("{:?}", order);
    }

    Ok(())
}
