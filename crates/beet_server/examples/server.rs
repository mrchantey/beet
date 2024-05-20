#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
	/* */
	beet_server::server::Server::default().run().await
}
