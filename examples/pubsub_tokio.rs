use amqp_manager::prelude::*;
use futures::FutureExt;
use tokio_amqp::LapinTokioExt;

#[tokio::main]
async fn main() {
    let pool_manager = LapinConnectionManager::new("amqp://guest:guest@127.0.0.1:5672//", &ConnectionProperties::default().with_tokio());
    let pool = r2d2::Pool::builder()
        .max_size(2)
        .build(pool_manager)
        .expect("Should build amqp connection pool");
    let amqp_manager = AmqpManager::new(pool).expect("Should create AmqpManager instance");
    let amqp_session = amqp_manager.get_session().await.expect("Should create AmqpSession instance");

    let queue_name = "queue-name";
    let create_queue_op = CreateQueue {
        queue_name: queue_name.to_string(),
        options: QueueDeclareOptions {
            auto_delete: false,
            ..Default::default()
        },
        ..Default::default()
    };
    amqp_session.create_queue(create_queue_op.clone()).await.expect("create_queue");

    amqp_session
        .publish_to_queue(PublishToQueue {
            queue_name: queue_name.to_string(),
            payload: Bytes::from_static(b"Hello world!"),
            ..Default::default()
        })
        .await
        .expect("publish_to_queue");

    amqp_session
        .create_consumer(
            CreateConsumer {
                queue_name: queue_name.to_string(),
                consumer_name: "consumer-name".to_string(),
                ..Default::default()
            },
            |delivery: DeliveryResult| async {
                if let Ok(Some((channel, delivery))) = delivery {
                    channel
                        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                        .map(|_| ())
                        .await;
                }
            },
        )
        .await
        .expect("create_consumer");

    let queue = amqp_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "Messages has been consumed");
}
