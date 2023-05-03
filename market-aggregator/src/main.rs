use futures::StreamExt;
use merge_streams::MergeStreams;
use services::{binance::{BinanceStream}, bitstamp::BitstampStream, OrderBookStream};

mod services;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut binance_client = BinanceStream::new("btcusdt", 10, 100)
        .await
        .unwrap();

    let mut bitstamp_client = BitstampStream::new("btcusdt")
        .await
        .unwrap();

    let result = binance_client.fetch_snapshot("btcusdt").await.unwrap();
    let result2 = bitstamp_client.fetch_snapshot("btcusdt").await.unwrap();

    let mut binance = binance_client
        .get_ob_stream()
        .await
        .unwrap();
    let mut bitstamp = bitstamp_client
        .get_ob_stream()
        .await
        .unwrap();
    
    // let mut final_stream = (binance, bitstamp).merge();

    while let Some(order) = bitstamp.next().await {
        println!("{:?}", order);
    }

    Ok(())
}
