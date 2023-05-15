use amqprs::{connection::{Connection, OpenConnectionArguments}, callbacks::{DefaultConnectionCallback, DefaultChannelCallback}, channel::{QueueDeclareArguments, QueueBindArguments, BasicConsumeArguments}};
use clap::Parser;
use queue::OrderbookSubscriber;
use server::{orderbook::orderbook_aggregator_server::OrderbookAggregatorServer, OrderbookService};
use tonic::transport::Server;

mod server;
mod queue;
mod types;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// URL to the RabbitMQ instance
    #[arg(short, long)]
    mq_host: String,

    /// Port to the RabbitMQ instance
    #[arg(short, long)]
    mq_port: u16,

    /// Username for the RabbitMQ instance
    #[arg(short, long)]
    mq_user: String,

    /// Password for the RabbitMQ user
    #[arg(short, long)]
    mq_pass: String,

    /// Name of the orderbook topic
    #[arg(default_value_t = String::from("orderbook"))]
    exchange_name: String,

    /// Queue name on RabbitMQ
    #[arg(short, long, default_value_t = String::from("grpc-server"))]
    queue_name: String,

    /// Port to run the gRPC server
    port: u16,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    let connection = Connection::open(&OpenConnectionArguments::new(
        &cli.mq_host,
        cli.mq_port,
        &cli.mq_user,
        &cli.mq_pass,
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
    let exchange_name = &cli.exchange_name;

    let (queue_name, _, _) = channel.queue_declare(QueueDeclareArguments::new(&cli.queue_name))
        .await?.unwrap();

    channel.queue_bind(QueueBindArguments::new(&queue_name, exchange_name, rounting_key))
        .await?;

    let args = BasicConsumeArguments::new(&queue_name, "grpc-server")
        .manual_ack(false)
        .finish();

    let orderbook_service = OrderbookService::new();
    let tx = orderbook_service.tx().clone();
    let sub = OrderbookSubscriber::new(tx);

    let _consumer = channel
        .basic_consume(sub, args)
        .await
        .unwrap();

    let address = format!("0.0.0.0:{}", cli.port).parse()?;
    
    Server::builder()
        .add_service(OrderbookAggregatorServer::new(orderbook_service))
        .serve(address)
        .await?;

    Ok(())
}
