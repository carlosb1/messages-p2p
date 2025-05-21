use std::time::Duration;
use rand::Rng;
use tokio::task::JoinHandle;
use messages_p2p::p2p;
use messages_p2p::p2p::bootstrap::BootstrapServer;
use messages_p2p::p2p::node::{ChatCommand};

pub fn generate_rand_msg() -> String {
    let mut rng = rand::rng();
    let random_number: u32 = rng.random_range(0..10000);
    format!("Random message: {}", random_number)
}


pub  fn run_relay_server() -> JoinHandle<()> {
    tokio::spawn(async {
        match BootstrapServer::new().await {
            Ok(mut server) => {
                if let Err(e) = server.run().await {
                    eprintln!("Server run failed: {:?}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to start server: {:?}", e);
            }
        }
    })
}

#[tokio::test]
async fn simple_node_communication() {

    let join_handle = run_relay_server();
    println!("Relay server started, waiting for 5 seconds to stabilize...");
    tokio::time::sleep(Duration::from_secs(5)).await;


    let node = p2p::node::run_node().unwrap();
    let tx1 = node.lock().await.command_sender();

    let node2 = p2p::node::run_node().unwrap();
    let tx2 = node2.lock().await.command_sender();


    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = tx1.send(ChatCommand::Publish("chat-room".to_string(),generate_rand_msg().into_bytes())).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = tx2.send(ChatCommand::Publish("chat-room".to_string(),generate_rand_msg().into_bytes())).await;
    let _ = tx2.send(ChatCommand::Publish("chat-room".to_string(),generate_rand_msg().into_bytes())).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

//    let _ = join_handle;

}