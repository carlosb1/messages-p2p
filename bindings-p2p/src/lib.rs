use std::cell::OnceCell;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
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
static NODE_TX: OnceLock<mpsc::Sender<ChatCommand>> = OnceLock::new();

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
pub fn start(server_address: String, peer_id: String, username: String) {
    init_logging();

    let bootstrap_config = messages_p2p::p2p::config::BootstrapConfig {
        peer_id: peer_id.clone(),
        address: server_address.clone(),
    };
    let config = messages_p2p::p2p::config::Config { bootstrap: bootstrap_config };
    let handler = MyEventHandler::default();
    log::debug!("Starting process for running service to connect to the network with peer id: {:?} \
    and server address: {:?}", peer_id.clone(), server_address.clone());
    thread::spawn(move || {
        if let Err(e) = std::panic::catch_unwind(|| {
            log::debug!("Creating Tokio runtime for node");
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                log::debug!("Setting blocking code and node tx");
                let mut node = ClientNode::new(config, handler).expect("Failed to create node");
                if NODE_TX.get().is_none() {
                    NODE_TX.set(node.command_sender()).expect("Failed to set NODE_TX");
                    log::debug!("NODE_TX initialized");
                } else {
                    log::warn!("NODE_TX was already initialized — skipping set");
                }
                node.run().await.expect("Node run failed");
            });
        }) {
            log::error!("Thread panicked: {:?}", e);
        }
    });
}

pub fn set_listener(listener: Arc<dyn EventListener>) {
    let _ = LISTENER.set(listener);
}


pub fn send_message(topic: String, message: String) {
    log::info!("Sending message: {} to topic: {}", message, topic);

    if let Some(tx) = NODE_TX.get() {
        let tx = tx.clone();
        thread::spawn(move || {
            if let Err(e) = std::panic::catch_unwind(move || {
                let rt = Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    let cmd = ChatCommand::Publish(topic.clone(), message.clone().into_bytes());
                    if let Err(e) = tx.send(cmd).await {
                        log::error!("Failed to send command: {:?}", e);
                    } else {
                        log::info!("Message sent successfully!");
                    }
                });
            }) {
                log::error!("Thread panicked while sending: {:?}", e);
            }
        });
    } else {
        log::warn!("NODE_TX not initialized. Did you forget to call `start()`?");
    }
}


uniffi::include_scaffolding!("bindings_p2p");