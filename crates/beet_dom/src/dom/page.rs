use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use serde_json::Value;

pub struct Page {
	provider: Box<dyn PageProvider>,
}

impl Page {
	pub async fn connect() -> Result<(Self, DefaultClient)> {
		let client = DefaultClient::connect().await?;
		(client.page(), client).xok()
	}

	pub fn new(provider: impl PageProvider) -> Self {
		Self {
			provider: Box::new(provider),
		}
	}

	pub async fn visit(&mut self, url: &str) -> Result<&mut Self> {
		self.provider.visit(url).await?;
		self.xok()
	}
	pub async fn current_url(&mut self) -> Result<String> {
		self.provider.current_url().await
	}

	pub async fn export_pdf(&mut self) -> Result<Bytes> {
		self.provider.export_pdf().await
	}
}


pub trait PageProvider: 'static + Send + Sync {
	fn visit(&self, url: &str) -> SendBoxedFuture<Result<()>>;
	fn current_url(&self) -> SendBoxedFuture<Result<String>>;

	/// Evaluate arbitary javascript, serializing the returned json script
	fn eval_async(
		&self,
		script: &str,
		args: Vec<Value>,
	) -> SendBoxedFuture<Result<Value>>;

	fn export_pdf(&self) -> SendBoxedFuture<Result<Bytes>>;
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let (mut page, client) = Page::connect().await.unwrap();
		page.visit("https://google.com").await.unwrap();
		page.current_url()
			.await
			.unwrap()
			.xpect_eq("https://www.google.com/");
		client.kill().await.unwrap();
	}

	#[sweet::test]
	#[ignore = "todo"]
	async fn export_pdf() {
		let (mut page, client) = Page::connect().await.unwrap();

		// let bytes = export_pdf("https://google.com").await.unwrap();
		// let _devtools = ChromeDevTools::spawn().await.unwrap();
		let bytes = page
			.visit("https://google.com")
			// .visit("https://beetstack.dev")
			.await
			.unwrap()
			.export_pdf()
			.await
			.unwrap();

		// let bytes = export_pdf("https://beetstack.dev").await.unwrap();
		bytes.len().xpect_greater_than(100);
		client.kill().await.unwrap();
	}
}
