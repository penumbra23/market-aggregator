use orderbook::{Empty, Summary, orderbook_aggregator_client::OrderbookAggregatorClient};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // creating a channel ie connection to server
    let channel = tonic::transport::Channel::from_static("http://localhost:9090")
        .connect()
        .await?;
    // creating gRPC client from channel
    let mut client = OrderbookAggregatorClient::new(channel);

    // creating a new Request
    let request = tonic::Request::new(
        Empty {},
    );
    let mut response = client.book_summary(request).await?.into_inner();

    // listening to stream
    while let Some(res) = response.message().await? {
        let best_ask = res.asks.first().unwrap().price;
        let best_bid = res.bids.first().unwrap().price;

        if best_ask < best_bid {
            println!("bid = {:?}, ask = {:?}", res.bids[0].price, res.asks[0].price);
        }
        
    }
    Ok(())
}