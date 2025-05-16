use std::path::PathBuf;
use futures::StreamExt;
use libp2p::{gossipsub, identity, kad, noise, PeerId, Swarm, tcp, yamux};
use libp2p::kad::Mode;
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use tokio::{io, select};
use tokio::io::AsyncBufReadExt;
use tracing_subscriber::EnvFilter;
use crate::p2p::behaviours::{build_gossipsub_behaviour, build_kademlia_behaviour};
use crate::p2p::config::{Config, load_config, print_config, save_config};
use crate::p2p::config;


#[derive(NetworkBehaviour)]
struct BootstrapNodeBehaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    gossipsub: gossipsub::Behaviour,
}

fn handle_input_line(kademlia: &mut kad::Behaviour<MemoryStore>, line: String) {
    println!("line={:?}", line);
}


pub struct BootstrapServer {
    keypair: identity::Keypair,
    peer_id: PeerId,
    swarm: Swarm<BootstrapNodeBehaviour>,
}

impl BootstrapServer {
    pub async fn new() -> anyhow::Result<Self> {
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();


        // Build behaviours
        let mut gossipsub = build_gossipsub_behaviour(&keypair)?;
        gossipsub.subscribe(&config::DEFAULT_TOPIC)?;

        let behaviour = |key: &identity::Keypair| -> BootstrapNodeBehaviour {
            BootstrapNodeBehaviour {
                kademlia: build_kademlia_behaviour(key),
                gossipsub,
            }
        };

        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
            .with_behaviour(behaviour)?
            .build();

        Ok(Self {
            keypair,
            peer_id,
            swarm,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {:?} and saving server config", address);
                    save_config(&self.peer_id, address)?;
                }
                /* event for messages */
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                                                                                 propagation_source: peer_id,
                                                                                 message_id: id,
                                                                                 message,
                                                                             })) => println!(
                    "Got message: '{}' with id: {id} from peer: {peer_id}",
                    String::from_utf8_lossy(&message.data),
                ),

                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Kademlia(kad::Event::InboundRequest { request })) => {
                    println!("🔗 FIND NODE command detected");
                    println!("request={:?}", request);
                },
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { result, ..})) => {
                    match result {
                        kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders { key, providers, .. })) => {
                            for peer in providers {
                                println!(
                                    "Peer {peer:?} provides key {:?}",
                                    std::str::from_utf8(key.as_ref()).unwrap()
                                );
                            }
                        },
                        _ => { println!("🔗 Other Kademlia event detected= {:?}", result); }
                    }
                }
                _ => {}
            }
        }
    }
}
