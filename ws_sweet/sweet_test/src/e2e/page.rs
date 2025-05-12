use crate::prelude::*;
use fantoccini::Client;


#[derive(Debug)]
pub struct Page {
	pub client: Client,
}

impl Page {
	/// create a new page
	pub fn new(client: Client) -> Self { Self { client } }
	/// Await the closure of the page, this will be triggered
	/// automatically when the page is dropped
	pub async fn close(self) { self.client.close().await.unwrap(); }
}


impl AsRef<Page> for Page {
	fn as_ref(&self) -> &Page { self }
}
impl std::ops::Deref for Page {
	type Target = Client;
	fn deref(&self) -> &Self::Target { &self.client }
}

impl<T: AsRef<Page>> Matcher<T> {
	/// Assert that the page has the given URL.
	/// Webdriver often appends a trailing slash so this will be removed if it is present
	pub async fn to_have_url(&self, url: &str) {
		let value = self.value.as_ref();
		let mut received = value
			.client
			.current_url()
			.await
			.unwrap()
			.as_str()
			.to_string();
		if received.ends_with('/') {
			received.pop();
		}
		self.assert_correct_with_received(
			received == url,
			&format!("to be '{}'", url),
			&received.as_str(),
		);
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	async fn works() {
		let page = visit("https://example.com").await;
		expect(&page).to_have_url("https://example.com").await;
		expect(&page).not().to_have_url("https://foobar.com").await;
	}
}
