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
//!     let pool_manager = LapinConnectionManager::new("amqp://guest:guest@127.0.0.1:5672//", &ConnectionProperties::default().with_tokio());
//!     let pool = r2d2::Pool::builder()
//!         .max_size(2)
//!         .build(pool_manager)
//!         .expect("Should build amqp connection pool");
//!     let amqp_manager = AmqpManager::new(pool).expect("Should create AmqpManager instance");
//!     let amqp_session = amqp_manager.get_session().await.expect("Should create AmqpSession instance");
//!
//!     let queue_name = "queue-name";
//!     let create_queue_op = CreateQueue {
//!         queue_name: queue_name.to_string(),
//!         options: QueueDeclareOptions {
//!             auto_delete: false,
//!             ..Default::default()
//!         },
//!         ..Default::default()
//!     };
//!     amqp_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
//!
//!     amqp_session
//!         .publish_to_queue(PublishToQueue {
//!             queue_name: queue_name.to_string(),
//!             payload: Bytes::from_static(b"Hello world!"),
//!             ..Default::default()
//!         })
//!         .await
//!         .expect("publish_to_queue");
//!
//!     amqp_session
//!         .create_consumer(
//!             CreateConsumer {
//!                 queue_name: queue_name.to_string(),
//!                 consumer_name: "consumer-name".to_string(),
//!                 ..Default::default()
//!             },
//!             |delivery: DeliveryResult| async {
//!                 if let Ok(Some((channel, delivery))) = delivery {
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

use lapin::message::Delivery;
use lapin::options::{BasicNackOptions, ConfirmSelectOptions};
use lapin::protocol::{AMQPError, AMQPErrorKind, AMQPHardError};
use lapin::types::{LongString, ShortString};
use r2d2_lapin::prelude::*;

use crate::prelude::{AMQPValue, FieldTable};
use crate::session::AmqpSession;

pub mod prelude;

mod ops;
mod session;

/// A type alias of the lapin's `Result` type.
pub type AmqpResult<T> = lapin::Result<T>;
/// A helper `Result` that can come handy to handle the consumer's message handler's errors.
pub type AmqpConsumerResult<T> = std::result::Result<T, AmqpConsumerError>;

/// The error type returned in the `AmqpConsumerResult<T>` struct.
#[derive(Debug)]
pub enum AmqpConsumerError {
    RecoverableError(String),
    UnrecoverableError(String),
    DuplicatedEventError(String),
}

impl AmqpConsumerError {
    /// A helper method that returns a `BasicNackOptions` instance depending on the error type.
    /// Example: `channel.basic_nack(delivery_tag, error::nack_options()).map(|_| ()).await`.
    pub fn nack_options(error: &Self) -> BasicNackOptions {
        match error {
            Self::RecoverableError(_) => BasicNackOptions {
                multiple: false,
                requeue: true,
            },
            Self::UnrecoverableError(_) => BasicNackOptions {
                multiple: false,
                requeue: false,
            },
            Self::DuplicatedEventError(_) => BasicNackOptions {
                multiple: false,
                requeue: false,
            },
        }
    }
}

/// The struct that handles the connection pool.
#[derive(Debug)]
pub struct AmqpManager {
    conn_pool: r2d2::Pool<LapinConnectionManager>,
}

impl AmqpManager {
    pub fn new(conn_pool: r2d2::Pool<LapinConnectionManager>) -> AmqpResult<Self> {
        Ok(Self { conn_pool })
    }

    /// Gets a new connection from the connection pool and creates a new channel.
    /// Both the connection and the channel will be closed when dropping the `AmqpSession` instance.
    pub async fn get_session(&self) -> AmqpResult<AmqpSession> {
        let conn = self.conn_pool.get().map_err(|e| {
            lapin::Error::ProtocolError(AMQPError::new(
                AMQPErrorKind::Hard(AMQPHardError::CONNECTIONFORCED),
                ShortString::from(e.to_string()),
            ))
        })?;
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

    /// Helper method to deserialize the `Delivery` contents into a `T` struct.
    pub fn deserialize_delivery<T: serde::de::DeserializeOwned>(delivery: &Delivery) -> AmqpConsumerResult<T> {
        let as_string = Self::delivery_to_string(&delivery.data)?;
        Self::deserialize_data(&as_string)
    }

    fn delivery_to_string(delivery_data: &[u8]) -> AmqpConsumerResult<String> {
        match String::from_utf8(delivery_data.to_vec()) {
            Ok(x) => Ok(x),
            Err(_) => {
                let msg = "Failed parsing delivery data".to_string();
                Err(AmqpConsumerError::UnrecoverableError(msg))
            }
        }
    }

    fn deserialize_data<T: serde::de::DeserializeOwned>(data: &str) -> AmqpConsumerResult<T> {
        match serde_json::from_str(data) {
            Ok(x) => Ok(x),
            Err(_) => {
                let msg = "Failed deserializing delivery data into struct".to_string();
                Err(AmqpConsumerError::UnrecoverableError(msg))
            }
        }
    }
}
