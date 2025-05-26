#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::thread;
    use std::sync::atomic::{AtomicBool, Ordering};
    use rand::Rng;
    use bindings_p2p::{Event, EventListener, init_logging, send_message, set_listener, start};

    const TEST_PEER_ID: &str = "12D3KooWPFwYxQzNrZd5oYTUFiczPstsGMepKwYBkZCBDMt85kz9";
    const TEST_USERNAME: &str = "carlosb";
    const TEST_ADDRESS: &str = "/ip4/172.17.0.1/tcp/34291";
    const TEST_TOPIC: &str = "chat-room";
    struct TestListener {
        called: Arc<AtomicBool>,
        last_message: Arc<Mutex<Option<String>>>,
    }


    pub fn generate_rand_msg() -> String {
        let mut rng = rand::rng();
        let random_number: u32 = rng.random_range(0..10000);
        format!("Random message: {}", random_number)
    }

    impl EventListener for TestListener {
        fn on_event(&self, event: Event) -> String {
            self.called.store(true, Ordering::SeqCst);
            let mut lock = self.last_message.lock().unwrap();
            *lock = Some(event.message.clone());
            "ok".to_string()
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_listener_receives_message() {
        init_logging();

        let called = Arc::new(AtomicBool::new(false));
        let last_message = Arc::new(Mutex::new(None));

        let listener = Arc::new(TestListener {
            called: called.clone(),
            last_message: last_message.clone(),
        });

        // Register the listener
        set_listener(listener.clone());



        // Start the node (can use dummy values)
        start(TEST_ADDRESS.to_string(), TEST_PEER_ID.to_string(),TEST_USERNAME.to_string());

        // Wait a bit for the node to initialize
        tokio::time::sleep(Duration::from_secs(1)).await;


        // Send a test message
        send_message(TEST_TOPIC.to_string(), "hello world".to_string() + generate_rand_msg().as_str());;

        // Wait for the message to be processedS
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Assertions
 //       assert!(called.load(Ordering::SeqCst), "Listener was not called");
        let message = last_message.lock().unwrap().clone();
        assert!(message.is_some(), "Listener did not receive a message");
    }
}
