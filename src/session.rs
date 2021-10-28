use lapin::publisher_confirm::Confirmation;
use lapin::{Channel, Consumer, ConsumerDelegate, Queue};

use crate::ops::*;
use crate::AmqpResult;

/// Manages amqp objects and its operations (exchanges, queues, consumers).
/// Since it only contains a `Channel` instance, this struct is also thread-safe.
///
/// Note that this struct doesn't implement `Clone`. This is done to encourage the user
/// to clone an instance of `AmqpManager` and create different sessions on different threads.
/// Otherwise, if the user shares an AmqpSession between threads and its channel is closed,
/// they won't be able to create a new one.
///
/// Refer to the [RabbitMQ channels docs](https://www.rabbitmq.com/channels.html#basics) for more information.
#[derive(Debug)]
pub struct AmqpSession {
    channel: Channel,
}

impl AmqpSession {
    pub(crate) fn new(channel: Channel) -> Self {
        Self { channel }
    }

    pub async fn create_exchange(&self, op: CreateExchange<'_>) -> AmqpResult<()> {
        Ok(self
            .channel
            .exchange_declare(op.exchange_name, op.kind, op.options, op.args)
            .await?)
    }

    pub async fn publish_to_exchange(&self, op: PublishToExchange<'_>) -> AmqpResult<Confirmation> {
        Ok(self
            .channel
            .basic_publish(op.exchange_name, op.routing_key, op.options, op.payload.to_vec(), op.properties)
            .await?
            .await?)
    }

    pub async fn delete_exchanges(&self, op: DeleteExchanges<'_>) -> AmqpResult<()> {
        for name in op.exchange_names {
            self.channel.exchange_delete(name, op.options).await?;
        }
        Ok(())
    }

    pub async fn create_queue(&self, op: CreateQueue<'_>) -> AmqpResult<Queue> {
        Ok(self.channel.queue_declare(op.queue_name, op.options, op.args).await?)
    }

    pub async fn bind_queue_to_exchange(&self, op: BindQueueToExchange<'_>) -> AmqpResult<()> {
        Ok(self
            .channel
            .queue_bind(op.queue_name, op.exchange_name, op.routing_key, op.options, op.args)
            .await?)
    }

    pub async fn publish_to_routing_key(&self, op: PublishToRoutingKey<'_>) -> AmqpResult<Confirmation> {
        Ok(self
            .channel
            .basic_publish("", op.routing_key, op.options, op.payload.to_vec(), op.properties)
            .await?
            .await?)
    }

    pub async fn delete_queues(&self, op: DeleteQueues<'_>) -> AmqpResult<()> {
        for name in op.queue_names {
            self.channel.queue_delete(name, op.options).await?;
        }
        Ok(())
    }

    pub async fn create_consumer(&self, op: CreateConsumer<'_>) -> AmqpResult<Consumer> {
        Ok(self
            .channel
            .basic_consume(op.queue_name, op.consumer_name, op.options, op.args)
            .await?)
    }

    pub async fn create_consumer_with_delegate<D: ConsumerDelegate + 'static>(
        &self,
        op: CreateConsumer<'_>,
        delegate: D,
    ) -> AmqpResult<Consumer> {
        let consumer = self
            .channel
            .basic_consume(op.queue_name, op.consumer_name, op.options, op.args)
            .await?;
        consumer.set_delegate(delegate)?;
        Ok(consumer)
    }

    pub async fn cancel_consumers(&self, op: CancelConsumers<'_>) -> AmqpResult<()> {
        for name in op.consumers_names {
            self.channel.basic_cancel(name, op.options).await?;
        }
        Ok(())
    }
}
