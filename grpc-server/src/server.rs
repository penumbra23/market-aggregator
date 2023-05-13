use async_trait::async_trait;
use log::error;
use tokio::sync::{broadcast::{Sender, Receiver, channel}, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use orderbook::{Empty, Summary, orderbook_aggregator_server::OrderbookAggregator};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

pub struct OrderbookService {
    tx: Sender<Result<Summary, Status>>,
    rx: Receiver<Result<Summary, Status>>,
}

impl OrderbookService {
    pub fn new() -> Self {
        let (tx, rx) = channel(10);
        Self {
            tx, 
            rx,
        }
    }

    pub fn tx(&self) -> Sender<Result<Summary, Status>> {
        self.tx.clone()
    }
}

#[async_trait]
impl OrderbookAggregator for OrderbookService {
    type BookSummaryStream = ReceiverStream<Result<Summary, Status>>;

    async fn book_summary(&self, _request: Request<Empty>) -> Result<Response<Self::BookSummaryStream>, Status> {
        let (tx, rx) = mpsc::channel(10);
        let mut client_recv = self.tx.subscribe();
        tokio::spawn(async move {
            while let Ok(order) = client_recv.recv().await {
                if let Err(err) = tx.send(order).await {
                    error!("Send error: {}", err);
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}