
use std::hash::{Hash};
use std::time::Duration;
use messages_p2p::p2p::node::{ChatCommand, ClientNode};


#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let mut node = ClientNode::new()?;
    let tx = node.command_sender();

    // Spawn the node's run loop
    let mut node_runner = node;
    tokio::spawn(async move {
        node_runner.run().await.expect("Failing for running node in background");
    });

    // Now spawn a task to publish messages
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        println!("🟢 Publishing messages from other task");
        for i in 0..10 {
            let msg = format!("Hi from other task! {i}");
            tx.send(ChatCommand::Publish("chat-room".into(), msg.into_bytes()))
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // Optionally: prevent main from exiting early
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }

}
