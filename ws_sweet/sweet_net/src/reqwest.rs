use once_cell::sync::Lazy;
use reqwest::Client;


static REQWEST_CLIENT: Lazy<Client> = Lazy::new(|| Client::new());


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
	use sweet_test::prelude::*;
	use sweet_utils::prelude::*;

	#[sweet_test::test]
	async fn works() {
		ReqwestClient::client()
			.get("https://example.com")
			.send()
			.await
			.xmap(expect)
			.to_be_ok();
	}
}
