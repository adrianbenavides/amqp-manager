#![forbid(unsafe_code)]
//! [Lapin](https://github.com/CleverCloud/lapin) wrapper that encapsulates the connections/channels
//! handling making it easier to use and less error prone.
//!
//! Here is an example using the tokio runtime:
//! ```no_run
//! use amqp_manager::prelude::*;
//! use futures::FutureExt;
//! use tokio_amqp::LapinTokioExt;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize, Serialize)]
//! struct SimpleDelivery(String);
//!
//! #[tokio::main]
//! async fn main() {
//!     let pool_manager = AmqpConnectionManager::new("amqp://guest:guest@127.0.0.1:5672//".to_string(), ConnectionProperties::default().with_tokio());
//!     let pool = mobc::Pool::builder().build(pool_manager);
//!     let amqp_manager = AmqpManager::new(pool).expect("Should create AmqpManager instance");
//!     let amqp_session = amqp_manager.get_session().await.expect("Should create AmqpSession instance");
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
//!             payload: Payload::new(&SimpleDelivery("Hello World".to_string())).unwrap(),
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
use lapin::{Channel, Connection, ConnectionProperties, ConnectionState, Error as LapinError};
use mobc::async_trait;
use mobc::Manager;
use thiserror::Error;

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
#[derive(Clone, Debug, Error)]
pub enum AmqpConsumerError {
    #[error("amqp-consumer recoverable error: `{0}`")]
    RecoverableError(String),
    #[error("amqp-consumer unrecoverable error: `{0}`")]
    UnrecoverableError(String),
    #[error("amqp-consumer duplicated event error: `{0}`")]
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
#[derive(Clone)]
pub struct AmqpManager {
    conn_pool: mobc::Pool<AmqpConnectionManager>,
}

impl AmqpManager {
    pub fn new(conn_pool: mobc::Pool<AmqpConnectionManager>) -> AmqpResult<Self> {
        Ok(Self { conn_pool })
    }

    /// Gets a new connection from the connection pool and creates a new channel.
    /// Both the connection and the channel will be closed when dropping the `AmqpSession` instance.
    pub async fn get_session(&self) -> AmqpResult<AmqpSession> {
        Ok(AmqpSession::new(self.get_channel().await?))
    }

    pub async fn get_session_with_confirm_select(&self) -> AmqpResult<AmqpSession> {
        let channel = self.get_channel().await?;
        channel.confirm_select(ConfirmSelectOptions::default()).await?;
        Ok(AmqpSession::new(channel))
    }

    async fn get_channel(&self) -> AmqpResult<Channel> {
        let conn = self.conn_pool.get().await.map_err(|e| {
            lapin::Error::ProtocolError(AMQPError::new(
                AMQPErrorKind::Hard(AMQPHardError::CONNECTIONFORCED),
                ShortString::from(e.to_string()),
            ))
        })?;
        conn.create_channel().await
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

    /// Helper method to deserialize the `Delivery` contents into a `T` struct with an arbitrary deserializer.
    pub fn deserialize_delivery<'de, T, F, Err>(delivery: &'de Delivery, deserializer: F) -> AmqpConsumerResult<T>
    where
        T: serde::de::Deserialize<'de>,
        F: FnOnce(&'de [u8]) -> Result<T, Err>,
        Err: std::error::Error,
    {
        match deserializer(&delivery.data) {
            Ok(x) => Ok(x),
            Err(_) => {
                let msg = "Failed deserializing delivery data into struct".to_string();
                Err(AmqpConsumerError::UnrecoverableError(msg))
            }
        }
    }

    /// Helper method to deserialize the `Delivery` contents into a `T` struct that was previously serialized as a json.
    pub fn deserialize_json_delivery<'de, T>(delivery: &'de Delivery) -> AmqpConsumerResult<T>
    where
        T: serde::de::Deserialize<'de>,
    {
        Self::deserialize_delivery(delivery, serde_json::from_slice::<'de, T>)
    }
}

#[derive(Clone, Debug)]
pub struct AmqpConnectionManager {
    addr: String,
    connection_properties: ConnectionProperties,
}

impl AmqpConnectionManager {
    pub fn new(addr: String, connection_properties: ConnectionProperties) -> Self {
        Self {
            addr,
            connection_properties,
        }
    }
}

#[async_trait]
impl Manager for AmqpConnectionManager {
    type Connection = Connection;
    type Error = LapinError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Connection::connect(self.addr.as_str(), self.connection_properties.clone()).await
    }

    async fn check(&self, conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        match conn.status().state() {
            ConnectionState::Connected => Ok(conn),
            other_state => Err(LapinError::InvalidConnectionState(other_state)),
        }
    }
}
