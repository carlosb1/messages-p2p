use crate::p2p::bootstrap::BootstrapServer;

pub mod p2p;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut server = BootstrapServer::new().await?;
    server.run().await
}


