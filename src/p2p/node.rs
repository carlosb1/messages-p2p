use std::sync::Arc;
use std::time::Duration;
use libp2p::{
    identity, PeerId, Multiaddr,
    gossipsub::{self, IdentTopic as Topic},
    kad::{self, store::MemoryStore},
    noise, tcp, yamux,
    swarm::{Swarm, SwarmEvent, NetworkBehaviour},
};
use tokio::sync::{mpsc, Mutex};
use futures::StreamExt;
use crate::p2p::config::{Config, load_config, print_config, DEFAULT_TOPIC};
use crate::p2p::behaviours::{build_gossipsub_behaviour, build_kademlia_behaviour};
use crate::p2p::handlers::{MessageHandler, SimpleClientHandler};

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub gossipsub: gossipsub::Behaviour,
}

#[derive(Debug, Clone)]
pub enum ChatCommand {
    Subscribe(String),
    Publish(String, Vec<u8>),
    Quit,
}

pub struct ClientNode<H: MessageHandler> {
    peer_id: PeerId,
    swarm: Swarm<NodeBehaviour>,
    command_rx: mpsc::Receiver<ChatCommand>,
    command_tx: mpsc::Sender<ChatCommand>,
    handler: H
}

pub fn run_node() -> anyhow::Result<Arc<Mutex<ClientNode<SimpleClientHandler>>>> {
    let handler = crate::p2p::handlers::SimpleClientHandler::default();
    let mut node = std::sync::Arc::new(Mutex::new(ClientNode::new(handler)?));

    // Spawn the background node task
    let node_runner = node.clone();
    tokio::spawn(async move {
        node_runner
            .lock()
            .await
            .run()
            .await
            .expect("Background node failed");
    });
    Ok(node.clone())
}

impl<H: MessageHandler> ClientNode<H>{
    pub fn from_config(node_config: Config, handler: H) -> anyhow::Result<Self> {
        // bootstrap info
        let server_peer_id: PeerId = node_config.bootstrap.peer_id.parse()?;
        let server_addr: Multiaddr = node_config.bootstrap.address.parse()?;

        println!("Server node config: ");
        print_config(&server_peer_id, Some(&server_addr), None);

        // client identity
        let client_keypair = identity::Keypair::generate_ed25519();
        let client_peer_id = client_keypair.public().to_peer_id();

        let mut gossipsub = build_gossipsub_behaviour(&client_keypair)?;
        gossipsub.subscribe(&DEFAULT_TOPIC)?;

        let behaviour = |key: &identity::Keypair| {
            Ok(NodeBehaviour {
                kademlia: build_kademlia_behaviour(key),
                gossipsub,
            })
        };

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(client_keypair.clone())
            .with_tokio()
            .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
            .with_behaviour(|key| behaviour(key))?
            .build();

        swarm.behaviour_mut().kademlia.add_address(&server_peer_id, server_addr.clone());
        swarm.dial(server_addr.clone())?;
        swarm.behaviour_mut().kademlia.bootstrap()?;

        let (tx, rx) = mpsc::channel::<ChatCommand>(32); // save tx if needed outside

        Ok(Self {
            peer_id: client_peer_id,
            swarm,
            command_rx: rx,
            command_tx: tx.clone(),
            handler
        })
    }


    pub fn from_config_path(path: String, handler: H) -> anyhow::Result<Self> {
        let node_config = load_config(Some(path))?;
        Self::from_config(node_config, handler)
    }
    pub fn new(handler: H) -> anyhow::Result<Self> {
        let node_config = load_config(None)?;
        Self::from_config(node_config, handler)
    }

    pub fn command_sender(&self) -> mpsc::Sender<ChatCommand> {
        self.command_tx.clone()
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let peer_id_str = self.peer_id.to_string();
        let mut tx = self.command_sender();

        loop {
            tokio::select! {
                /* manage commands to call */
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        ChatCommand::Subscribe(topic_name) => {
                            let topic = Topic::new(topic_name);
                            if self.swarm.behaviour_mut().gossipsub.subscribe(&topic).is_ok() {
                                println!("✅ Subscribed to topic: {topic}");
                            }
                        }
                        ChatCommand::Publish(topic_name, msg) => {
                            let topic = Topic::new(topic_name);
                            println!("🟢 Publishing: {} with topic {:?}", String::from_utf8_lossy(&msg), topic.clone());
                            if self.swarm.behaviour_mut().gossipsub.publish(topic.clone(), msg).is_ok() {
                                println!("📤 Published to topic: {topic}");
                            }
                        }
                        other => {
                            println!("❌ Command not supported {:?}", other);
                        }
                    }
                }
                event = self.swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            println!("🧩 Listening on: {:?}", address);
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            println!("✅ Connected to: {peer_id}");
                        }
                        SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message { propagation_source, message_id, message })) => {
                            let response_command = self.handler.handle_message(propagation_source,&message.data.clone());
                            if let Some(command) = response_command {
                                if let Err(er) = self.command_tx.send(command.clone()).await {
                                    println!("❌ Failed to send command: {er}");;
                                }
                            } else {
                                println!(
                                    "📨 Got message: '{}' from {propagation_source} (id: {message_id})",
                                    String::from_utf8_lossy(&message.data.clone())
                                );
                            }

                        }
                        SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(event)) => {
                            println!("🧠 Kademlia event: {:?}", event);
                        }
                        other => {
                            println!("🔎 Other event: {:?}", other);
                        }
                    }
                }
            }
        }
    }
}
