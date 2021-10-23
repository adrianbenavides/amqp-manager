#![forbid(unsafe_code)]
//! [Lapin](https://github.com/CleverCloud/lapin) wrapper that encapsulates the connections/channels
//! handling making it easier to use and less error prone.
//!
//! Here is an example using the tokio runtime:
//! ```no_run
//! use amqp_manager::prelude::*;
//! use deadpool_lapin::{Config, Runtime};
//! use futures::FutureExt;
//!
//! #[tokio::main]
//! async fn main() {
//!     let pool = Config {
//!        url: Some("amqp://guest:guest@127.0.0.1:5672//".to_string()),
//!         ..Default::default()
//!     }
//!     .create_pool(Some(Runtime::Tokio1))
//!     .expect("Should create DeadPool instance");
//!     let manager = AmqpManager::new(pool);
//!     let session = manager
//!         .create_session_with_confirm_select()
//!         .await
//!         .expect("Should create AmqpSession instance");
//!
//!     let queue_name = "queue-name";
//!     let create_queue_op = CreateQueue {
//!         queue_name: &queue_name,
//!         options: QueueDeclareOptions {
//!             auto_delete: false,
//!             ..Default::default()
//!         },
//!         ..Default::default()
//!     };
//!     session.create_queue(create_queue_op.clone()).await.expect("create_queue");
//!
//!     session
//!         .publish_to_routing_key(PublishToRoutingKey {
//!             routing_key: &queue_name,
//!             payload: "Hello World".as_bytes(),
//!             ..Default::default()
//!         })
//!         .await
//!         .expect("publish_to_queue");
//!
//!     session
//!         .create_consumer_with_delegate(
//!             CreateConsumer {
//!                 queue_name: &queue_name,
//!                 consumer_name: "consumer-name",
//!                 ..Default::default()
//!             },
//!             |delivery: DeliveryResult| async {
//!                 if let Ok(Some((channel, delivery))) = delivery {
//!                    let payload = std::str::from_utf8(&delivery.data).unwrap();
//!                     assert_eq!(payload, "Hello World");
//!                     channel
//!                         .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
//!                         .map(|_| ())
//!                         .await;
//!                 }
//!             },
//!         )
//!         .await
//!         .expect("create_consumer");
//!
//!     let queue = session.create_queue(create_queue_op.clone()).await.expect("create_queue");
//!     assert_eq!(queue.message_count(), 0, "Messages has been consumed");
//! }
//! ```

use thiserror::Error;

use crate::prelude::lapin::options::ConfirmSelectOptions;
use crate::prelude::lapin::types::{LongString, ShortString};
use crate::prelude::{AMQPValue, FieldTable};
use crate::session::AmqpSession;

pub mod prelude;

mod ops;
mod session;

pub type AmqpResult<T> = Result<T, AmqpError>;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AmqpError {
    #[error(transparent)]
    Lapin(#[from] lapin::Error),
    #[error(transparent)]
    Pool(#[from] deadpool_lapin::PoolError),
}

/// This struct contains a connection pool. Since a connection can be used to create multiple channels,
/// it's recommended to use a pool with a low number of connections.
/// Refer to the [RabbitMQ channels docs](https://www.rabbitmq.com/channels.html#basics) for more information.
#[derive(Debug, Clone)]
pub struct AmqpManager {
    pool: deadpool_lapin::Pool,
}

impl AmqpManager {
    pub fn new(pool: deadpool_lapin::Pool) -> Self {
        Self { pool }
    }

    /// Creates a new channel using a connection from the pool.
    /// The channel will be closed when dropping the `AmqpSession` instance.
    /// Creating a new connection is slower than creating a channel, so it's better
    /// to reuse the connection and create as many channels as needed out of that a connection.
    pub async fn create_session(&self) -> AmqpResult<AmqpSession> {
        let conn = self.pool.get().await?;
        let channel = conn.create_channel().await?;
        Ok(AmqpSession::new(channel))
    }

    /// Creates a new channel using a connection from the pool. This channel can be awaited to receive confirms.
    pub async fn create_session_with_confirm_select(&self) -> AmqpResult<AmqpSession> {
        let conn = self.pool.get().await?;
        let channel = conn.create_channel().await?;
        channel.confirm_select(ConfirmSelectOptions::default()).await?;
        Ok(AmqpSession::new(channel))
    }

    /// Helper method to create a `FieldTable` instance with the dead-letter argument.
    pub fn dead_letter_args(args: FieldTable, dead_letter_exchange_name: &str) -> FieldTable {
        let mut args = args;
        args.insert(
            ShortString::from("x-dead-letter-exchange"),
            AMQPValue::LongString(LongString::from(dead_letter_exchange_name)),
        );
        args
    }
}
