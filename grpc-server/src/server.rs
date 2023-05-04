use async_trait::async_trait;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use orderbook::{Empty, Summary, orderbook_aggregator_server::OrderbookAggregator};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

#[derive(Default)]
pub struct OrderbookService {}

#[async_trait]
impl OrderbookAggregator for OrderbookService {
    type BookSummaryStream = ReceiverStream<Result<Summary, Status>>;

    async fn book_summary(&self, request: Request<Empty>) -> Result<Response<Self::BookSummaryStream>, Status> {
        todo!()
    }
}