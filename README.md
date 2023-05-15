# Rust Market Aggregator

Simple orderbook aggregator that maintains a local orderbook fetched from different exchanges.

Currently supported exchanges are:
- Binance
- Bitstamp

## Architecture

![diagram](./assets/diagram.png)

- WS Market Aggregator - maintains websocket connection with each exchange, reconnects on stream errors and maintains a local orderbook from each exchange

`market-aggregator --mq-host <MQ_HOST> --mq-port <MQ_PORT> --mq-user <MQ_USER> --mq-pass <MQ_PASS> <EXCHANGE_NAME> <TRADING_PAIR>`

- gRPC Streaming Server - consumes orders from the queue and server them on a gRPC stream

`grpc-server --mq-host <MQ_HOST> --mq-port <MQ_PORT> --mq-user <MQ_USER> --mq-pass <MQ_PASS> -q <QUEUE_NAME> <EXCHANGE_NAME> <PORT>`

## Running

To run all the services inside Docker, just run the `docker-compose.yaml`:

`docker compose up -d`

## Consuming

The repo also contains an example client that subscribes to the gRCP stream. To run the example, make sure the services are running inside Docker and run:

`cargo run --bin client`

## License
MIT