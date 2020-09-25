use lapin::publisher_confirm::Confirmation;
use lapin::{Channel, Consumer, ConsumerDelegate, Queue};

use crate::ops::*;
use crate::AmqpResult;

/// The struct used to manage amqp objects (exchanges, queues, consumers).
/// Since it only contains a `Channel` instance, this struct is also thread-safe.
#[derive(Debug, Clone)]
pub struct AmqpSession {
    channel: Channel,
}

impl AmqpSession {
    pub(crate) fn new(channel: Channel) -> Self {
        Self { channel }
    }

    pub async fn create_exchange(&self, op: CreateExchange) -> AmqpResult<()> {
        self.channel.exchange_declare(&op.exchange_name, op.kind, op.options, op.args).await
    }

    pub async fn publish_to_exchange(&self, op: PublishToExchange) -> AmqpResult<Confirmation> {
        self.channel
            .basic_publish(&op.exchange_name, "", op.options, op.payload.to_vec(), op.properties)
            .await?
            .await
    }

    pub async fn delete_exchanges(&self, op: DeleteExchanges) -> AmqpResult<()> {
        for name in &op.exchange_names {
            self.channel.exchange_delete(name, op.options).await?;
        }
        Ok(())
    }

    pub async fn create_queue(&self, op: CreateQueue) -> AmqpResult<Queue> {
        self.channel.queue_declare(&op.queue_name, op.options, op.args).await
    }

    pub async fn bind_queue_to_exchange(&self, op: BindQueueToExchange) -> AmqpResult<()> {
        self.channel
            .queue_bind(&op.queue_name, &op.exchange_name, "", op.options, op.args.clone())
            .await
    }

    pub async fn publish_to_queue(&self, op: PublishToQueue) -> AmqpResult<Confirmation> {
        self.channel
            .basic_publish("", &op.queue_name, op.options, op.payload.to_vec(), op.properties)
            .await?
            .await
    }

    pub async fn delete_queues(&self, op: DeleteQueues) -> AmqpResult<()> {
        for name in &op.queue_names {
            self.channel.queue_delete(name, op.options).await?;
        }
        Ok(())
    }

    pub async fn create_consumer<D: ConsumerDelegate + 'static>(&self, op: CreateConsumer, message_handler: D) -> AmqpResult<Consumer> {
        let consumer = self
            .channel
            .basic_consume(&op.queue_name, &op.consumer_name, op.options, op.args)
            .await?;
        consumer.set_delegate(message_handler)?;
        Ok(consumer)
    }

    pub async fn cancel_consumers(&self, op: CancelConsumers) -> AmqpResult<()> {
        for name in &op.consumers_names {
            self.channel.basic_cancel(name, op.options).await?;
        }
        Ok(())
    }
}
