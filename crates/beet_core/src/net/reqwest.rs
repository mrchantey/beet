use reqwest::Client;
use std::sync::LazyLock;


static REQWEST_CLIENT: LazyLock<Client> = LazyLock::new(|| Client::new());


/// A wrapper around the reqwest client. Calling `Client::new()` creates
/// a new client, which is expensive. This wrapper creates a single
/// static client and returns a reference to it.
pub struct ReqwestClient;

impl ReqwestClient {
	/// Returns a reference to the static reqwest client.
	pub fn client() -> &'static Client { &*REQWEST_CLIENT }
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	#[ignore = "flaky example.com"]
	async fn works() {
		ReqwestClient::client()
			.get("https://example.com")
			.send()
			.await
			.xmap(expect)
			.to_be_ok();
	}
}
