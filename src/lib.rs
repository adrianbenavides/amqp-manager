#![forbid(unsafe_code)]
//! [Lapin](https://github.com/CleverCloud/lapin) wrapper that encapsulates the connections/channels
//! handling making it easier to use and less error prone.
//!
//! Here is an example using the tokio runtime:
//! ```no_run
//! use amqp_manager::prelude::*;
//! use futures::FutureExt;
//! use tokio_amqp::LapinTokioExt;
//!
//! #[tokio::main]
//! async fn main() {
//!     let conn = Connection::connect("amqp://guest:guest@127.0.0.1:5672//", ConnectionProperties::default().with_tokio()).await.unwrap();
//!     let amqp_session = AmqpManager::get_session(&conn).await.expect("Should create AmqpSession instance");
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
//!     amqp_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
//!
//!     amqp_session
//!         .publish_to_routing_key(PublishToRoutingKey {
//!             routing_key: &queue_name,
//!             payload: "Hello World".as_bytes(),
//!             ..Default::default()
//!         })
//!         .await
//!         .expect("publish_to_queue");
//!
//!     amqp_session
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
//!     let queue = amqp_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
//!     assert_eq!(queue.message_count(), 0, "Messages has been consumed");
//! }
//! ```

use crate::prelude::lapin::options::ConfirmSelectOptions;
use crate::prelude::lapin::types::{LongString, ShortString};
use crate::prelude::{AMQPValue, Connection, FieldTable};
use crate::session::AmqpSession;

pub mod prelude;

mod ops;
mod session;

/// Type alias of the lapin's `Result` type.
pub type AmqpResult<T> = lapin::Result<T>;

/// Helper struct to create channels.
#[derive(Clone, Default)]
pub struct AmqpManager;

impl AmqpManager {
    /// Creates a new channel using the given connection.
    /// The channel will be closed when dropping the `AmqpSession` instance.
    /// Creating a new connection is slower than creating a channel, so it's better to reuse the connection and create
    /// as many channels out of that single connection as needed.
    pub async fn get_session(conn: &Connection) -> AmqpResult<AmqpSession> {
        let channel = conn.create_channel().await?;
        Ok(AmqpSession::new(channel))
    }

    /// Creates a new channel using the given connection. This channel can be awaited to receive confirms.
    pub async fn get_session_with_confirm_select(conn: &Connection) -> AmqpResult<AmqpSession> {
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
