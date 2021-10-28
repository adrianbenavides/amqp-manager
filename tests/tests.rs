use amqp_manager::prelude::*;
use deadpool_lapin::{Config, Runtime};

#[tokio::test]
async fn pub_sub() {
    let session = utils::get_amqp_session().await;
    let queue_name = utils::generate_random_name();
    let create_queue_op = CreateQueue {
        queue_name: &queue_name,
        options: QueueDeclareOptions {
            auto_delete: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let queue = session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "No messages to consume");

    let messages_to_queue = 100_u32;
    for _ in 0..messages_to_queue {
        let confirmation = session
            .publish_to_routing_key(PublishToRoutingKey {
                routing_key: &queue_name,
                payload: &utils::dummy_payload(),
                ..Default::default()
            })
            .await
            .expect("publish_to_queue");
        assert!(confirmation.is_ack());
    }

    let queue = session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), messages_to_queue, "There are messages ready to be consumed");

    session
        .create_consumer_with_delegate(
            CreateConsumer {
                queue_name: &queue_name,
                consumer_name: &utils::generate_random_name(),
                ..Default::default()
            },
            utils::dummy_delegate,
        )
        .await
        .expect("create_consumer");

    let queue = session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "Messages has been consumed");
}

#[tokio::test]
async fn broadcast() {
    let session = utils::get_amqp_session().await;
    let exchange_name = &utils::generate_random_name();

    session
        .create_exchange(CreateExchange {
            exchange_name,
            kind: ExchangeKind::Fanout,
            options: ExchangeDeclareOptions {
                auto_delete: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .await
        .expect("create_exchange");

    let create_queue_op = CreateQueue {
        options: QueueDeclareOptions {
            auto_delete: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let queue_names: Vec<String> = (0..5).map(|_| utils::generate_random_name()).collect();
    for queue_name in &queue_names {
        let mut create_queue_op = create_queue_op.clone();
        create_queue_op.queue_name = queue_name;
        session.create_queue(create_queue_op).await.expect("create_queue");
        session
            .bind_queue_to_exchange(BindQueueToExchange {
                queue_name,
                exchange_name,
                ..Default::default()
            })
            .await
            .expect("bind_queue_to_exchange");
    }

    let messages_to_queue = 100_u32;
    for _ in 0..messages_to_queue {
        let confirmation = session
            .publish_to_exchange(PublishToExchange {
                exchange_name,
                payload: &utils::dummy_payload(),
                ..Default::default()
            })
            .await
            .expect("publish_to_exchange");
        assert!(confirmation.is_ack());
    }

    for queue_name in &queue_names {
        let mut create_queue_op = create_queue_op.clone();
        create_queue_op.queue_name = queue_name;
        let queue = session.create_queue(create_queue_op).await.expect("create_queue");
        assert_eq!(queue.message_count(), messages_to_queue, "There are messages ready to be consumed");
    }

    for queue_name in &queue_names {
        session
            .create_consumer_with_delegate(
                CreateConsumer {
                    queue_name,
                    consumer_name: &utils::generate_random_name(),
                    ..Default::default()
                },
                utils::dummy_delegate,
            )
            .await
            .expect("create_consumer");
    }

    for queue_name in &queue_names {
        let mut create_queue_op = create_queue_op.clone();
        create_queue_op.queue_name = queue_name;
        let queue = session.create_queue(create_queue_op).await.expect("create_queue");
        assert_eq!(queue.message_count(), 0, "Messages has been consumed");
    }
}

#[tokio::test]
async fn reuse_connection_for_multiple_sessions() {
    let pool = Config {
        url: Some(utils::AMQP_URL.clone()),
        ..Default::default()
    }
    .create_pool(Some(Runtime::Tokio1))
    .expect("Failed to create pool");
    let manager = AmqpManager::new(pool);
    let session = manager.create_session().await.expect("create_session");
    let queue_name = utils::generate_random_name();
    let create_queue_op = CreateQueue {
        queue_name: &queue_name,
        options: QueueDeclareOptions {
            auto_delete: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let queue = session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "No messages to consume");

    let mut handles = Vec::with_capacity(10);
    for _session_idx in 0..10 {
        let queue_name = queue_name.clone();
        let manager = manager.clone();
        handles.push(tokio::spawn(async move {
            let session = manager.create_session().await.expect("create_session");
            for _msg_idx in 0..10 {
                session
                    .publish_to_routing_key(PublishToRoutingKey {
                        routing_key: &queue_name,
                        payload: &utils::dummy_payload(),
                        ..Default::default()
                    })
                    .await
                    .expect("publish_to_queue");
            }
        }));
    }
    futures::future::try_join_all(handles)
        .await
        .expect("Failed awaiting session future");
}

#[tokio::test]
async fn multiple_connections_with_multiple_sessions() {
    let pool = Config {
        url: Some(utils::AMQP_URL.clone()),
        ..Default::default()
    }
    .create_pool(Some(Runtime::Tokio1))
    .expect("Failed to create pool");
    let manager = AmqpManager::new(pool);

    let session = manager.create_session().await.expect("create_session");
    let queue_name = utils::generate_random_name();
    let create_queue_op = CreateQueue {
        queue_name: &queue_name,
        options: QueueDeclareOptions {
            auto_delete: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let queue = session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "No messages to consume");

    let max_conns = deadpool_lapin::PoolConfig::default().max_size;
    let mut handles = Vec::with_capacity(10 * max_conns);
    for _conn_idx in 0..max_conns {
        for _session_idx in 0..10 {
            let queue_name = queue_name.clone();
            let manager = manager.clone();
            handles.push(tokio::spawn(async move {
                let session = manager.create_session().await.expect("create_session");
                for _msg_idx in 0..10 {
                    session
                        .publish_to_routing_key(PublishToRoutingKey {
                            routing_key: &queue_name,
                            payload: &utils::dummy_payload(),
                            ..Default::default()
                        })
                        .await
                        .expect("publish_to_queue");
                }
            }));
        }
    }
    futures::future::try_join_all(handles)
        .await
        .expect("Failed awaiting session future");
}

mod utils {
    use futures::FutureExt;
    use once_cell::sync::Lazy;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    use super::*;

    pub(crate) static AMQP_URL: Lazy<String> = Lazy::new(|| {
        dotenv::dotenv().ok();
        std::env::var("TEST_AMQP_URL").unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672//".to_string())
    });

    pub(crate) async fn get_amqp_session() -> AmqpSession {
        let pool = Config {
            url: Some(AMQP_URL.clone()),
            ..Default::default()
        }
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create pool");
        let manager = AmqpManager::new(pool);
        manager
            .create_session_with_confirm_select()
            .await
            .expect("Failed to create session")
    }

    const DUMMY_DELIVERY_CONTENTS: &str = "Hello world!";

    #[derive(serde::Deserialize, serde::Serialize)]
    struct DummyDelivery(String);

    pub fn dummy_payload() -> Vec<u8> {
        serde_json::to_vec(&DummyDelivery(DUMMY_DELIVERY_CONTENTS.to_string())).unwrap()
    }

    pub(crate) async fn dummy_delegate(delivery: DeliveryResult) {
        if let Ok(Some((channel, delivery))) = delivery {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            let payload: DummyDelivery = serde_json::from_slice(&delivery.data).expect("Should deserialize the delivery");
            assert_eq!(&payload.0, DUMMY_DELIVERY_CONTENTS);
            channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .map(|_| ())
                .await;
        }
    }

    pub(crate) fn generate_random_name() -> String {
        thread_rng().sample_iter(&Alphanumeric).take(10).map(char::from).collect::<String>()
    }
}
