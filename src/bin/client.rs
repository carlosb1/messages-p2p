#![feature(slice_range)]

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;
use libp2p::{kad, mdns, Multiaddr, noise, PeerId, tcp, yamux};
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::gossipsub;
use libp2p::gossipsub::IdentTopic as Topic;
use tokio::io;
use tokio::sync::Mutex;

const str_peer_id: &str = "12D3KooWCzuQwXP9LLek6zrxzLD9msHxyKUNaYSANygDXHnDSXef";

#[derive(NetworkBehaviour)]
struct Behaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    mdns: mdns::tokio::Behaviour,
    gossipsub: gossipsub::Behaviour,
}
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let peer_id: PeerId = str_peer_id.parse().unwrap();
    let addr: Multiaddr = "/ip4/127.0.0.1/tcp/33841".parse().unwrap(); //TODO take care of this loopback address
    println!("#########################################################");
    println!("Server peer id: {:?}", peer_id.clone());
    println!("Server address: {:?}", addr.clone());
    println!("#########################################################");

    let topic: Topic = Topic::new("chat-room");

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            println!("#########################################################");
            println!("Client Public key: {:?}", key.public().clone());
            println!("Client PeerId: {:?}", key.public().clone().to_peer_id());
            println!("#########################################################");

            // To content-address message, we can take the hash of message and use it as an ID.


            Ok(Behaviour {
                kademlia: kad::Behaviour::new(
                    key.public().to_peer_id(),
                    MemoryStore::new(key.public().to_peer_id()),
                ),
                mdns: mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?,
                gossipsub
            })
        })?.build();

    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    // Set a custom gossipsub configuration
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
        .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message
        // signing)
        .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
        .build()
        .map_err(io::Error::other)?; // Temporary hack because `build` does not return a proper `std::error::Error`.

    // build a gossipsub network behaviour
    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )?;


    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
    swarm.dial(addr.clone()).unwrap(); // Opcional pero recomendado
    swarm.behaviour_mut().kademlia.bootstrap().unwrap();



    /* set up general chat */
    let gossipsub = swarm.behaviour_mut().gossipsub;
    gossipsub.subscribe(&topic).unwrap();

    // Shareable Gossipsub publisher
    let gossipsub_shared = Arc::new(Mutex::new(gossipsub));

    let gossipsub_for_task = gossipsub_shared.clone();
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;

        println!("🟢 Publishing messages from other task");
        for _ in 0..10 {
            let msg = format!("Hi from other task!");
            let mut gs = gossipsub_for_task.lock().await;
            if let Err(e) = gs.publish(topic.clone(), msg.as_bytes()) {
                eprintln!("Error publishing: {:?}", e);
            } else {
                println!("🟢 Published message");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

    });


    loop {
        match swarm.select_next_some().await {
            /*  received responses messages */
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { result, .. })) => {
                match result {
                    kad::QueryResult::GetClosestPeers(Ok(ok)) => {
                        if ok.peers.is_empty() {
                            println!("❌ No peers found in the DHT.");
                        } else {
                            println!("✅ Found peers:");
                            for peer in ok.peers {
                                println!("  - {:?}", peer);
                            }
                        }
                    }
                    kad::QueryResult::GetClosestPeers(Err(err)) => {
                        eprintln!("❌ Failed to query closest peers: {:?}", err);
                    }
                    _ => {
                        println!("🔎 Otro evento de consulta: {:?}", result);
                    }
                }
            }
            /* kademlia event */
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(event)) => {
                println!("Kademlia event: {:?}", event);
            }
            /* connection established */
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("✅ Conectado con {peer_id}");
            }
            /* event for messages */
            SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
                                                                  propagation_source: peer_id,
                                                                  message_id: id,
                                                                  message,
                                                              })) => println!(
                "Got message: '{}' with id: {id} from peer: {peer_id}",
                String::from_utf8_lossy(&message.data),
            ),

            other => {
                println!("🔎 Otro evento: {:?}", other);
            }
        }
    }
}