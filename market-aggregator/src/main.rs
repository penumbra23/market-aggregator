use amqprs::{connection::{Connection, OpenConnectionArguments}, channel::{ExchangeDeclareArguments, ExchangeType}};
use app::App;

use clap::Parser;
use services::{binance::BinanceStream, bitstamp::BitstampStream};

mod services;
mod app;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// URL to the RabbitMQ instance
    #[arg(short, long)]
    mq_host: String,

    /// URL to the RabbitMQ instance
    #[arg(short, long)]
    mq_port: u16,

    /// URL to the RabbitMQ instance
    #[arg(short, long)]
    mq_user: String,

    /// URL to the RabbitMQ instance
    #[arg(short, long)]
    mq_pass: String,

    /// Name of the orderbook topic
    #[arg(default_value_t = String::from("orderbook"))]
    exchange_name: String,

    /// Trading pair to watch for
    pair: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    let conn_args = OpenConnectionArguments::new(
        &cli.mq_host,
        cli.mq_port,
        &cli.mq_user,
        &cli.mq_pass
    );
    
    let connection = Connection::open(&conn_args)
        .await?;

    let channel = connection.open_channel(None).await.unwrap();

    channel.exchange_declare(
            ExchangeDeclareArguments::of_type(&cli.exchange_name, ExchangeType::Topic)
            .durable(true)
            .finish()
        ).await?;
    
    let binance_client = BinanceStream::new(&cli.pair)
        .await
        .unwrap();
    
    let bitstamp_client = BitstampStream::new(&cli.pair)
        .await
        .unwrap();

    let mut app = App::new(channel);

    app.add_client("binance", Box::new(binance_client))?;
    app.add_client("bitstamp", Box::new(bitstamp_client))?;

    app.run().await?;

    Ok(())
}
