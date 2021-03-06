//! These are the structs used as arguments to execute the session's operations.

use lapin::options::{
    BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions, ExchangeDeleteOptions, QueueBindOptions,
    QueueDeclareOptions, QueueDeleteOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};
use std::fmt::Debug;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreateExchange<'a> {
    pub exchange_name: &'a str,
    pub kind: ExchangeKind,
    pub options: ExchangeDeclareOptions,
    pub args: FieldTable,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishToExchange<'a> {
    pub exchange_name: &'a str,
    pub routing_key: &'a str,
    pub options: BasicPublishOptions,
    pub payload: &'a [u8],
    pub properties: BasicProperties,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeleteExchanges<'a> {
    pub exchange_names: &'a [&'a str],
    pub options: ExchangeDeleteOptions,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreateQueue<'a> {
    pub queue_name: &'a str,
    pub options: QueueDeclareOptions,
    pub args: FieldTable,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BindQueueToExchange<'a> {
    pub queue_name: &'a str,
    pub exchange_name: &'a str,
    pub routing_key: &'a str,
    pub options: QueueBindOptions,
    pub args: FieldTable,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishToRoutingKey<'a> {
    pub routing_key: &'a str,
    pub options: BasicPublishOptions,
    pub payload: &'a [u8],
    pub properties: BasicProperties,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeleteQueues<'a> {
    pub queue_names: &'a [&'a str],
    pub options: QueueDeleteOptions,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreateConsumer<'a> {
    pub queue_name: &'a str,
    pub consumer_name: &'a str,
    pub options: BasicConsumeOptions,
    pub args: FieldTable,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CancelConsumers<'a> {
    pub consumers_names: &'a [&'a str],
    pub options: BasicCancelOptions,
}
