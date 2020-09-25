//! These are the structs used as arguments to execute the session's operations.

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
    pub payload: Bytes,
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
pub struct PublishToQueue {
    pub queue_name: String,
    pub options: BasicPublishOptions,
    pub payload: Bytes,
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
