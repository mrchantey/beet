use beet_net_axum::prelude::*;


#[tokio::main]
pub async fn main() -> anyhow::Result<()> { Server::default().run().await }
