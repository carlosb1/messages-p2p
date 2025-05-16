use std::fs;
use libp2p::{Multiaddr, PeerId};
use libp2p::gossipsub::IdentTopic;
use libp2p::identity::Keypair;
use serde::{Deserialize, Serialize};
use tokio::io;
use once_cell::sync::Lazy;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub bootstrap: BootstrapConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BootstrapConfig {
    pub peer_id: String,
    pub address: String,
}


pub static DEFAULT_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("chat-room"));
const DEFAULT_CONFIG: &'static str = "temp_config.toml";

pub fn save_config(peer_id: &PeerId, address: Multiaddr) -> anyhow::Result<()> {
    let bootstrap_config = BootstrapConfig{ peer_id: peer_id.clone().to_string(), address: address.clone().to_string() };
        let config = Config{bootstrap: bootstrap_config};
    fs::write(DEFAULT_CONFIG, toml::to_string(&config)?.as_str())?;
    Ok(())
}

pub fn load_config(path: Option<String>) -> anyhow::Result<Config> {
    match path {
        Some(path) => {
            let node_config: Config = toml::from_str(fs::read_to_string(path)?.as_str())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(node_config)
        }
        None => {
            let node_config: Config = toml::from_str(fs::read_to_string(DEFAULT_CONFIG)?.as_str())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(node_config)
        }
    }
}


pub fn print_config(peer_id: &PeerId, address: Option<&Multiaddr>, keys: Option<Keypair>) {
    println!("#########################################################");
    println!("PeerId: {:?}", peer_id);
    if let Some(address) = address {
        println!("Address: {:?}", address);
    }
    if let Some(keys) = keys {
        println!("Public key: {:?}", keys.public())
    }
    println!("#########################################################");
}