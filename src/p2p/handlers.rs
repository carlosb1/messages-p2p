use libp2p::PeerId;
use crate::p2p::node::ChatCommand;

pub trait MessageHandler: Send + 'static {
    fn handle_message(&mut self, peer: PeerId, data: &[u8])-> Option<ChatCommand>;
}


#[derive(Debug, Clone, Default)]
pub struct SimpleClientHandler;

impl MessageHandler for SimpleClientHandler {
    fn handle_message(&mut self, peer: PeerId, data: &[u8]) -> Option<ChatCommand>{
        println!("Node: received message from {}: {:?}", peer, String::from_utf8_lossy(data));
        None
    }
}

