[![docs](https://docs.rs/amqp-manager/badge.svg)](https://docs.rs/amqp-manager)
[![crates.io-version](https://img.shields.io/crates/v/amqp-manager)](https://crates.io/crates/amqp-manager)
[![tests](https://github.com/adrianbenavides/amqp-manager/workflows/Tests/badge.svg)](https://github.com/adrianbenavides/amqp-manager/actions)
[![audit](https://github.com/adrianbenavides/amqp-manager/workflows/Audit/badge.svg)](https://github.com/adrianbenavides/amqp-manager/actions)
[![crates.io-license](https://img.shields.io/crates/l/amqp-manager)](LICENSE)

[Lapin](https://github.com/CleverCloud/lapin) wrapper that encapsulates the use of connections/channels and provides some 
helpful methods making it easier to use and less error prone. 

## Usage

```rust
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
```

## Build-time Requirements

The crate is tested on `ubuntu-latest` against the following rust versions: nightly, beta, stable and 1.45.0.
It is possible that it works with older versions as well but this is not tested.
Please see the details of the lapin crate about its requirements.