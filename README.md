[![docs](https://docs.rs/amqp-manager/badge.svg)](https://docs.rs/amqp-manager)
[![crates.io-version](https://img.shields.io/crates/v/amqp-manager)](https://crates.io/crates/amqp-manager)
[![tests](https://github.com/adrianbenavides/amqp-manager/workflows/Tests/badge.svg)](https://github.com/adrianbenavides/amqp-manager/actions)
[![audit](https://github.com/adrianbenavides/amqp-manager/workflows/Audit/badge.svg)](https://github.com/adrianbenavides/amqp-manager/actions)
[![crates.io-license](https://img.shields.io/crates/l/amqp-manager)](LICENSE)

[Lapin](https://github.com/CleverCloud/lapin) wrapper that encapsulates the use of connections/channels and creation of amqp objects.

## Usage

```rust
use amqp_manager::prelude::*;
use deadpool_lapin::{Config, Runtime};
use futures::FutureExt;

#[tokio::main]
async fn main() {
    let pool = Config {
        url: Some("amqp://guest:guest@127.0.0.1:5672//".to_string()),
        ..Default::default()
    }
        .create_pool(Some(Runtime::Tokio1))
        .expect("Should create DeadPool instance");
    let manager = AmqpManager::new(pool);
    let session = manager
        .create_session_with_confirm_select()
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
```

## Build-time Requirements

Please see the details of the lapin and deadpool crates about their requirements.
