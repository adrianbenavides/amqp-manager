use amqp_manager::prelude::*;

#[tokio::test]
async fn pub_sub() {
    let tx_session = utils::get_amqp_session().await;

    let queue_name = utils::generate_random_name();
    let create_queue_op = CreateQueue {
        queue_name: &queue_name,
        options: QueueDeclareOptions {
            auto_delete: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let queue = tx_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "No messages to consume");

    let messages_to_queue = 100_u32;
    for _ in 0..messages_to_queue {
        let confirmation = tx_session
            .publish_to_routing_key(PublishToRoutingKey {
                routing_key: &queue_name,
                payload: &utils::dummy_payload(),
                ..Default::default()
            })
            .await
            .expect("publish_to_queue");
        assert!(confirmation.is_ack());
    }

    let queue = tx_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), messages_to_queue, "There are messages ready to be consumed");

    let rx_session = utils::get_amqp_session().await;
    rx_session
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

    let queue = tx_session.create_queue(create_queue_op.clone()).await.expect("create_queue");
    assert_eq!(queue.message_count(), 0, "Messages has been consumed");
}

#[tokio::test]
async fn broadcast() {
    let tx_session = utils::get_amqp_session().await;

    let exchange_name = utils::generate_random_name();
    tx_session
        .create_exchange(CreateExchange {
            exchange_name: &exchange_name,
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
        tx_session.create_queue(create_queue_op).await.expect("create_queue");
        tx_session
            .bind_queue_to_exchange(BindQueueToExchange {
                queue_name: &queue_name,
                exchange_name: &exchange_name,
                ..Default::default()
            })
            .await
            .expect("bind_queue_to_exchange");
    }

    let messages_to_queue = 100_u32;
    for _ in 0..messages_to_queue {
        let confirmation = tx_session
            .publish_to_exchange(PublishToExchange {
                exchange_name: &exchange_name,
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
        let queue = tx_session.create_queue(create_queue_op).await.expect("create_queue");
        assert_eq!(queue.message_count(), messages_to_queue, "There are messages ready to be consumed");
    }

    let rx_session = utils::get_amqp_session().await;
    for queue_name in &queue_names {
        rx_session
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
    }

    for queue_name in &queue_names {
        let mut create_queue_op = create_queue_op.clone();
        create_queue_op.queue_name = queue_name;
        let queue = tx_session.create_queue(create_queue_op).await.expect("create_queue");
        assert_eq!(queue.message_count(), 0, "Messages has been consumed");
    }
}

mod utils {
    use super::*;
    use futures::FutureExt;
    use once_cell::sync::Lazy;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use tokio_amqp::LapinTokioExt;

    static AMQP_URL: Lazy<String> = Lazy::new(|| {
        dotenv::dotenv().ok();
        std::env::var("TEST_AMQP_URL").unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672//".to_string())
    });

    static AMQP_MANAGER: Lazy<AmqpManager> = Lazy::new(|| {
        let manager = AmqpConnectionManager::new(AMQP_URL.clone(), ConnectionProperties::default().with_tokio());
        let pool = mobc::Pool::builder().max_open(2).build(manager);
        AmqpManager::new(pool).expect("Should create AmqpManager instance")
    });

    pub(crate) async fn get_amqp_session() -> AmqpSession {
        AMQP_MANAGER.get_session_with_confirm_select().await.unwrap()
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
