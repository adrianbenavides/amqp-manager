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
    let pool_manager = AmqpConnectionManager::new(
        "amqp://guest:guest@127.0.0.1:5672//".to_string(),
        ConnectionProperties::default().with_tokio(),
    );
    let pool = mobc::Pool::builder().max_open(2).build(pool_manager);
    let amqp_manager = AmqpManager::new(pool).expect("Should create AmqpManager instance");
    let tx_session = amqp_manager
        .get_session_with_confirm_select()
        .await
        .expect("Should create AmqpSession instance");

    let create_queue_op = CreateQueue {
        options: QueueDeclareOptions {
            auto_delete: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let queue = tx_session.create_queue(create_queue_op.clone()).await.expect("create_queue");

    let confirmation = tx_session
        .publish_to_routing_key(PublishToRoutingKey {
            routing_key: queue.name().as_str(),
            payload: "Hello World".as_bytes(),
            ..Default::default()
        })
        .await
        .expect("publish_to_queue");
    assert!(confirmation.is_ack());

    let rx_session = amqp_manager.get_session().await.unwrap();
    rx_session
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

    let queue = tx_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "Messages has been consumed");
}
```

## Build-time Requirements

The crate is tested on `ubuntu-latest` against the following rust versions: nightly, beta, stable and 1.45.0.
It is possible that it works with older versions as well but this is not tested.
Please see the details of the lapin crate about its requirements.
