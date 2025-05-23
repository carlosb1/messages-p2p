use std::cell::OnceCell;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use tokio::runtime::Runtime;
use uniffi::export;
use messages_p2p::p2p::node::ClientNode;
use messages_p2p::p2p::node::ChatCommand;
use messages_p2p::p2p::handlers::MessageHandler;
use messages_p2p::PeerId;

#[cfg(target_os = "android")]
pub fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
    log::info!("Logging initialized for Android");
}

#[cfg(not(target_os = "android"))]
pub fn init_logging() {
    let _ = env_logger::builder()
        .is_test(false)
        .filter_level(log::LevelFilter::Debug)// for testing
        .try_init();
    log::info!("Logging initialized for CLI/Desktop");
}


pub struct Event {
    pub topic: String,
    pub message: String,
}

static LISTENER: OnceLock<Arc<dyn EventListener>> = OnceLock::new();
static NODE: OnceLock<Arc<Mutex<ClientNode<MyEventHandler>>>> = OnceLock::new();

#[derive(Clone, Debug, Default)]
pub struct MyEventHandler;

impl MessageHandler for MyEventHandler {
    fn handle_message(&mut self, peer: PeerId, data: &[u8]) -> Option<ChatCommand>{
        log::info!("Node: received message from {}: {:?}", peer.clone(),
            String::from_utf8_lossy(data.clone()));
        match LISTENER.get() {
            Some(listener) => {
                let topic = "chat-room".to_string(); // FIXME you can replace this with real topic logic
                let msg = String::from_utf8_lossy(data).to_string();
                let event = Event{topic, message: msg};
                listener.on_event(event);  // ✅ here you call it
            },
            None => {
                log::info!("The listener is not activated");
            },

        }
        None
    }
}


pub trait EventListener: Send + Sync {
    fn on_event(&self, event: Event) -> String;
}
fn start() {
    init_logging();
    thread::spawn(|| {

        if let Err(e) = std::panic::catch_unwind(|| {
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async {
                log::info!("Initializing the client node...");
                let client = NODE.get_or_init(|| {
                    Arc::new(Mutex::new(ClientNode::new(MyEventHandler::default()).expect("It could not create the client node")))
                });
                log::info!("Running the client node...");
                client.lock().unwrap().run().await.unwrap();
            });
        }) {
            log::error!("Thread panicked: {:?}", e);
        }
    });

}
fn set_listener(listener: Arc<dyn EventListener>) {
    let _ = LISTENER.set(listener);
}

fn send_message(topic: String, message: String) {
    log::info!("Sending message: {} to topic: {}", message, topic);
    if let Some(client) = NODE.get() {
        let rt = Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            let tx = client.lock().unwrap().command_sender();
            tx.send(ChatCommand::Publish(topic, message.into_bytes())).await.unwrap();
        });
    }
}

uniffi::include_scaffolding!("bindings_p2p");