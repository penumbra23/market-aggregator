use amqprs::{connection::{Connection, OpenConnectionArguments}, callbacks::{DefaultConnectionCallback, DefaultChannelCallback}, channel::{QueueDeclareArguments, QueueBindArguments, BasicConsumeArguments}};
use queue::OrderbookSubscriber;
use server::{orderbook::orderbook_aggregator_server::OrderbookAggregatorServer, OrderbookService};
use tonic::transport::Server;

mod server;
mod queue;
mod types;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::open(&OpenConnectionArguments::new(
        "localhost",
        5672,
        "user",
        "password",
    ))
    .await?;
    connection
        .register_callback(DefaultConnectionCallback)
        .await?;
    
    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();
    channel
        .register_callback(DefaultChannelCallback)
        .await?;

    let rounting_key = "rate.*";
    let exchange_name = "orderbook";

    let (queue_name, _, _) = channel.queue_declare(QueueDeclareArguments::new("grpc-server"))
        .await?.unwrap();

    channel.queue_bind(QueueBindArguments::new(&queue_name, exchange_name, rounting_key))
        .await?;

    let args = BasicConsumeArguments::new(&queue_name, "grpc-server")
        .manual_ack(false)
        .finish();

    let mut consumer = channel
        .basic_consume(OrderbookSubscriber::new(), args)
        .await
        .unwrap();

    let address = "0.0.0.0:9090".parse().unwrap();
    let orderbook_service = OrderbookService::default();
    
    Server::builder()
        .add_service(OrderbookAggregatorServer::new(orderbook_service))
        .serve(address)
        .await?;

    Ok(())
}
