use std::{collections::HashMap};

use amqprs::{channel::{BasicPublishArguments, Channel}, BasicProperties};
use common::{Orderbook, OrderbookUpdate, OrderbookQueueItem};
use futures::{stream::select_all, StreamExt};
use log::{error, info};
use crate::services::{OrderbookError};

use crate::services::OrderbookConnection;

pub struct App {
    clients: HashMap<String, Box<dyn OrderbookConnection + Unpin>>,
    orderbooks: HashMap<String, Orderbook>,
    channel: Channel,
}

impl App {
    pub fn new(channel: Channel) -> Self {
        Self {
            clients: HashMap::new(),
            orderbooks: HashMap::new(),
            channel
        }
    }

    pub fn add_client(&mut self, stream: &str, client: Box<dyn OrderbookConnection + Unpin>) -> Result<(), OrderbookError> {
        if self.clients.contains_key(stream) {
            return Err(OrderbookError{ details: String::from("Client already present") });
        }

        self.clients.insert(stream.to_owned(), client);
        self.orderbooks.insert(stream.to_owned(), Orderbook::new(stream));

        Ok(())
    }

    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (_, cl) in &mut self.clients {
            let res = cl.connect().await;
            if let Err(err) = res {
                error!("Error on client connect: {}", err);
            }
        }

        let mut streams = self.clients
            .iter_mut()
            .map(|(_, v)| v)
            .collect::<Vec<&mut Box<dyn OrderbookConnection + Unpin>>>();

        while let Some(order) = select_all(streams.iter_mut()).next().await {
            let order_update: OrderbookUpdate = order;
            
            let orderbook = match self.orderbooks.get_mut(&order_update.stream) {
                Some(ob) => ob,
                None => {
                    info!("No orderbook registered for {}", order_update.stream);
                    continue;
                },
            };

            orderbook.update_book(&order_update);

            let args = BasicPublishArguments::new(
                "orderbook", 
                &format!("rate.{}", order_update.stream)
            );

            let queue_item: OrderbookQueueItem = orderbook.clone().into();

            self.channel
                .basic_publish(
                    BasicProperties::default().with_persistence(true).finish(),
                    serde_json::to_string(&queue_item)?.into_bytes(),
                    args)
                .await?;
        }
        Ok(())
    }
}