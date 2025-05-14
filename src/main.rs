use std::fs;
use libp2p::{gossipsub, identity, PeerId};

const DEFAULT_CONFIG_PATH: &str = "config.toml";

use serde::Deserialize;
use tracing_subscriber::EnvFilter;
use std::error::Error;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Read;
use std::time::Duration;

use futures::stream::StreamExt;
use libp2p::{
    kad,
    kad::{store::MemoryStore, Mode},
    mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use tokio::{
    io::{self, AsyncBufReadExt},
    select,
};


#[derive(Deserialize, Debug)]
struct Config {
    secret_key: Option<u8>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //let secret_key_seed = Some(0u8);
    let mut secret_key_seed: Option<u8> = None;


    let path =std::path::Path::new(DEFAULT_CONFIG_PATH);
    let fil_config = std::fs::File::open(path);

    /*
    if let Ok(mut file) = fil_config {
        let mut str_config = String::new();
        if let Ok(siz) = file.read_to_string(&mut str_config) {
            if let Ok(config) = toml::from_str::<Config>(str_config.as_ref()) {
                println!("Config: {:?}", config);
                secret_key_seed = config.secret_key;
            } else {
                println!("Error parsing config");
            }
        }
    }
    */


    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    // We create a custom network behaviour that combines Kademlia and mDNS.
    #[derive(NetworkBehaviour)]
    struct Behaviour {
        kademlia: kad::Behaviour<MemoryStore>,
        mdns: mdns::tokio::Behaviour,
        gossipsub: gossipsub::Behaviour,
    }

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            println!("#########################################################");
            println!("PeerId: {:?}", key.public().clone().to_peer_id());
            println!("#########################################################");

            // To content-address message, we can take the hash of message and use it as an ID.
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


            Ok(Behaviour {
                kademlia: kad::Behaviour::new(
                    key.public().to_peer_id(),
                    MemoryStore::new(key.public().to_peer_id()),
                ),
                mdns: mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?,
                gossipsub,
            })
        })?
        .build();

    swarm.behaviour_mut().kademlia.set_mode(Some(Mode::Server));


    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns.
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let my_generated_peer_id = swarm.local_peer_id();
    println!("generated_peer={:?}", my_generated_peer_id);
    // Kick it off.
    loop {
        select! {
        Ok(Some(line)) = stdin.next_line() => {
            handle_input_line(&mut swarm.behaviour_mut().kademlia, line);
        }
        event = swarm.select_next_some() => match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                let peer_id = swarm.local_peer_id();
                println!("Listening in {address:?}");
//                println!("Listening in {address:?}/p2p/{peer_id}");
            },
                /* event for messages */
            SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
                                                                  propagation_source: peer_id,
                                                                  message_id: id,
                                                                  message,
                                                              })) => println!(
                "Got message: '{}' with id: {id} from peer: {peer_id}",
                String::from_utf8_lossy(&message.data),
            ),

            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, multiaddr) in list {
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr);
                }
            },

            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::InboundRequest { request })) => {
                    println!("🔗 Otro nodo envió una petición a este nodo (por ejemplo, FIND_NODE)");
                    println!("request={:?}", request);
            },
           SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                ..
            })) => {
                println!("Query finished with closest peers: {:#?}", ok.peers);
                return Ok(());
            },
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed {
                result:
                    kad::QueryResult::GetClosestPeers(Err(kad::GetClosestPeersError::Timeout {
                        ..
                    })),
                ..
            })) => {
                println!("Query for closest peers timed out")
            },
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { result, ..})) => {
                match result {
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders { key, providers, .. })) => {
                        for peer in providers {
                            println!(
                                "Peer {peer:?} provides key {:?}",
                                std::str::from_utf8(key.as_ref()).unwrap()
                            );
                        }
                    },
                    kad::QueryResult::GetProviders(Err(err)) => {
                        eprintln!("Failed to get providers: {err:?}");
                    }
                    kad::QueryResult::GetRecord(Ok(
                        kad::GetRecordOk::FoundRecord(kad::PeerRecord {
                            record: kad::Record { key, value, .. },
                            ..
                        })
                    )) => {
                        println!(
                            "Got record {:?} {:?}",
                            std::str::from_utf8(key.as_ref()).unwrap(),
                            std::str::from_utf8(&value).unwrap(),
                        );
                    }
                    kad::QueryResult::GetRecord(Ok(_)) => {}
                    kad::QueryResult::GetRecord(Err(err)) => {
                        eprintln!("Failed to get record: {err:?}");
                    }
                    kad::QueryResult::PutRecord(Ok(kad::PutRecordOk { key })) => {
                        println!(
                            "Successfully put record {:?}",
                            std::str::from_utf8(key.as_ref()).unwrap()
                        );
                    }
                    kad::QueryResult::PutRecord(Err(err)) => {
                        eprintln!("Failed to put record: {err:?}");
                    }
                    kad::QueryResult::StartProviding(Ok(kad::AddProviderOk { key })) => {
                        println!(
                            "Successfully put provider record {:?}",
                            std::str::from_utf8(key.as_ref()).unwrap()
                        );
                    }
                    kad::QueryResult::StartProviding(Err(err)) => {
                        eprintln!("Failed to put provider record: {err:?}");
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        }
    }


    Ok(())

}

fn handle_input_line(kademlia: &mut kad::Behaviour<MemoryStore>, line: String) {
    println!("line={:?}", line);
}
