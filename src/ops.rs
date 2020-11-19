//! These are the structs used as arguments to execute the session's operations.

use crate::AmqpResult;
use bytes::Bytes;
use lapin::options::{
    BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions, ExchangeDeleteOptions, QueueBindOptions,
    QueueDeclareOptions, QueueDeleteOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};
use std::fmt::Debug;

#[derive(Debug, Default, Clone)]
pub struct CreateExchange {
    pub exchange_name: String,
    pub kind: ExchangeKind,
    pub options: ExchangeDeclareOptions,
    pub args: FieldTable,
}

#[derive(Debug, Default, Clone)]
pub struct PublishToExchange {
    pub exchange_name: String,
    pub options: BasicPublishOptions,
    pub payload: Payload,
    pub properties: BasicProperties,
}

#[derive(Debug, Default, Clone)]
pub struct DeleteExchanges {
    pub exchange_names: Vec<String>,
    pub options: ExchangeDeleteOptions,
}

#[derive(Debug, Default, Clone)]
pub struct CreateQueue {
    pub queue_name: String,
    pub options: QueueDeclareOptions,
    pub args: FieldTable,
}

#[derive(Debug, Default, Clone)]
pub struct BindQueueToExchange {
    pub queue_name: String,
    pub exchange_name: String,
    pub options: QueueBindOptions,
    pub args: FieldTable,
}

#[derive(Debug, Default, Clone)]
pub struct PublishToRoutingKey {
    pub routing_key: String,
    pub options: BasicPublishOptions,
    pub payload: Payload,
    pub properties: BasicProperties,
}

#[derive(Debug, Default, Clone)]
pub struct DeleteQueues {
    pub queue_names: Vec<String>,
    pub options: QueueDeleteOptions,
}

#[derive(Debug, Default, Clone)]
pub struct CreateConsumer {
    pub queue_name: String,
    pub consumer_name: String,
    pub options: BasicConsumeOptions,
    pub args: FieldTable,
}

#[derive(Debug, Default, Clone)]
pub struct CancelConsumers {
    pub consumers_names: Vec<String>,
    pub options: BasicCancelOptions,
}

#[derive(Debug, Default, Clone)]
pub struct Payload {
    contents: Bytes,
}

impl Payload {
    pub fn new<T: serde::Serialize>(contents: &T) -> AmqpResult<Self> {
        let serialized = serde_json::to_vec(contents).map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        Ok(Self {
            contents: Bytes::from(serialized),
        })
    }

    pub fn contents(&self) -> Bytes {
        self.contents.clone()
    }
}
