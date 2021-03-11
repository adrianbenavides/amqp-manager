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

use mobc::async_trait;
use mobc::Manager;

use crate::prelude::lapin::options::ConfirmSelectOptions;
use crate::prelude::lapin::types::{LongString, ShortString};
use crate::prelude::{AMQPValue, Channel, ConnectionProperties, FieldTable};
use crate::session::AmqpSession;

pub mod prelude;

mod ops;
mod session;

/// A type alias of the lapin's `Result` type.
pub type AmqpResult<T> = lapin::Result<T>;

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
        let conn = self
            .conn_pool
            .get()
            .await
            .map_err(|_mobc_err| lapin::Error::InvalidConnectionState(lapin::ConnectionState::Error))?;
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
    type Connection = lapin::Connection;
    type Error = lapin::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        lapin::Connection::connect(self.addr.as_str(), self.connection_properties.clone()).await
    }

    async fn check(&self, conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        match conn.status().state() {
            lapin::ConnectionState::Connected => Ok(conn),
            other_state => Err(lapin::Error::InvalidConnectionState(other_state)),
        }
    }
}
