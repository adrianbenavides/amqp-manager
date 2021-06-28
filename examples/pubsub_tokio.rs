use amqp_manager::prelude::*;
use futures::FutureExt;
use tokio_amqp::LapinTokioExt;

#[tokio::main]
async fn main() {
    let manager = AmqpManager::new("amqp://guest:guest@127.0.0.1:5672//", ConnectionProperties::default().with_tokio());
    let conn = manager.connect().await.unwrap();
    let session = AmqpManager::create_session_with_confirm_select(&conn)
        .await
        .expect("Should create AmqpSession instance");

    let create_queue_op = CreateQueue {
        options: QueueDeclareOptions {
            auto_delete: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let queue = session.create_queue(create_queue_op.clone()).await.expect("create_queue");

    let confirmation = session
        .publish_to_routing_key(PublishToRoutingKey {
            routing_key: queue.name().as_str(),
            payload: "Hello World".as_bytes(),
            ..Default::default()
        })
        .await
        .expect("publish_to_queue");
    assert!(confirmation.is_ack());

    session
        .create_consumer_with_delegate(
            CreateConsumer {
                queue_name: queue.name().as_str(),
                ..Default::default()
            },
            move |delivery: DeliveryResult| async {
                if let Ok(Some((channel, delivery))) = delivery {
                    let payload = std::str::from_utf8(&delivery.data).unwrap();
                    assert_eq!(payload, "Hello World");
                    channel
                        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                        .map(|_| ())
                        .await;
                }
            },
        )
        .await
        .expect("create_consumer");

    let queue = session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "Messages has been consumed");
}
