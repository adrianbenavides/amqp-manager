use amqp_manager::prelude::*;

#[tokio::test]
async fn pub_sub() {
    let amqp_session = utils::get_amqp_session().await;

    let queue_name = utils::generate_random_name();
    let create_queue_op = CreateQueue {
        queue_name: queue_name.to_string(),
        options: QueueDeclareOptions {
            auto_delete: false,
            ..Default::default()
        },
        ..Default::default()
    };

    let queue = amqp_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "No messages to consume");

    let messages_to_queue = 100_u32;
    for _ in 0..messages_to_queue {
        let confirmation = amqp_session
            .clone()
            .publish_to_queue(PublishToQueue {
                queue_name: queue_name.to_string(),
                payload: Bytes::from_static(b"Hello world!"),
                ..Default::default()
            })
            .await
            .expect("publish_to_queue");
        assert!(confirmation.is_ack());
    }

    let queue = amqp_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), messages_to_queue, "There are messages ready to be consumed");

    amqp_session
        .create_consumer(
            CreateConsumer {
                queue_name: queue_name.to_string(),
                consumer_name: utils::generate_random_name(),
                ..Default::default()
            },
            utils::dummy_message_handler,
        )
        .await
        .expect("create_consumer");

    let queue = amqp_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "Messages has been consumed");
}

#[tokio::test]
async fn broadcast() {
    let amqp_session = utils::get_amqp_session().await;

    let exchange_name = utils::generate_random_name();
    amqp_session
        .create_exchange(CreateExchange {
            exchange_name: exchange_name.to_string(),
            kind: ExchangeKind::Fanout,
            options: ExchangeDeclareOptions {
                auto_delete: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .await
        .expect("create_exchange");

    let create_queue_op = |queue_name: &str| CreateQueue {
        queue_name: queue_name.to_string(),
        options: QueueDeclareOptions {
            auto_delete: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let queue_names: Vec<String> = (0..5).map(|_| utils::generate_random_name()).collect();
    for queue_name in &queue_names {
        amqp_session.create_queue(create_queue_op(&queue_name)).await.expect("create_queue");
        amqp_session
            .bind_queue_to_exchange(BindQueueToExchange {
                queue_name: queue_name.clone(),
                exchange_name: exchange_name.clone(),
                ..Default::default()
            })
            .await
            .expect("bind_queue_to_exchange");
    }

    let messages_to_queue = 100_u32;
    for _ in 0..messages_to_queue {
        let confirmation = amqp_session
            .clone()
            .publish_to_exchange(PublishToExchange {
                exchange_name: exchange_name.clone(),
                payload: Bytes::from_static(b"Hello world!"),
                ..Default::default()
            })
            .await
            .expect("publish_to_exchange");
        assert!(confirmation.is_ack());
    }

    for queue_name in &queue_names {
        let queue = amqp_session.create_queue(create_queue_op(&queue_name)).await.expect("create_queue");
        assert_eq!(queue.message_count(), messages_to_queue, "There are messages ready to be consumed");
    }

    for queue_name in &queue_names {
        amqp_session
            .create_consumer(
                CreateConsumer {
                    queue_name: queue_name.to_string(),
                    consumer_name: utils::generate_random_name(),
                    ..Default::default()
                },
                utils::dummy_message_handler,
            )
            .await
            .expect("create_consumer");
    }

    for queue_name in &queue_names {
        let queue = amqp_session.create_queue(create_queue_op(&queue_name)).await.expect("create_queue");
        assert_eq!(queue.message_count(), 0, "Messages has been consumed");
    }
}

mod utils {
    use super::*;
    use futures::FutureExt;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use tokio_amqp::LapinTokioExt;

    lazy_static::lazy_static! {
        static ref AMQP_URL: String = {
            dotenv::dotenv().ok();
            std::env::var("TEST_AMQP_URL").unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672//".to_string())
        };
        static ref AMQP_MANAGER: AmqpManager = {
            let manager = LapinConnectionManager::new(&AMQP_URL, &ConnectionProperties::default().with_tokio());
            let pool = r2d2::Pool::builder().max_size(2).build(manager).expect("Should build amqp connection pool");
            AmqpManager::new(pool).expect("Should create AmqpManager instance")
        };
    }

    pub(crate) async fn get_amqp_session() -> AmqpSession {
        AMQP_MANAGER.get_session().await.unwrap()
    }

    pub(crate) async fn dummy_message_handler(delivery: DeliveryResult) {
        if let Ok(Some((channel, delivery))) = delivery {
            tokio::time::delay_for(tokio::time::Duration::from_millis(50)).await;
            channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .map(|_| ())
                .await;
        }
    }

    pub(crate) fn generate_random_name() -> String {
        thread_rng().sample_iter(&Alphanumeric).take(10).collect()
    }
}
