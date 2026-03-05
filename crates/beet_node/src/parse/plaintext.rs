use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::futures_lite::Stream;

#[derive(Debug, Default, Clone)]
pub struct PlainTextParser;

impl PlainTextParser {
	pub fn new() -> Self { Self::default() }
}

impl NodeParser for PlainTextParser {
	fn parse(
		&mut self,
		entity: AsyncEntity,
		bytes: impl AsRef<[u8]>,
	) -> impl Future<Output = Result> {
		let bytes = bytes.as_ref().to_vec();
		async move {
			let text = std::str::from_utf8(&bytes)?.to_string();
			entity
				.with_then(move |mut entity| {
					match entity.get_mut::<Value>() {
						Some(mut value) => {
							value.set_if_neq(Value::new(text));
						}
						None => {
							entity.insert(Value::new(text));
						}
					}
					Ok(())
				})
				.await
		}
	}

	fn parse_stream(
		&mut self,
		entity: AsyncEntity,
		stream: impl 'static + Unpin + Stream<Item = Result<impl AsRef<[u8]>>>,
	) -> impl Future<Output = Result> {
		let mut stream = stream_ext::bytes_to_text(stream);
		async move {
			entity.insert_then(Value::new(String::new())).await;
			while let Some(result) = stream.next().await {
				let text = result?;
				entity
					.get_mut::<Value, _>(move |mut value| {
						match value.as_mut() {
							Value::Str(existing) => {
								existing.push_str(&text);
								Ok(())
							}
							_ => bevybail!("Expected a String Value"),
						}
					})
					.await??;
			}

			Ok(())
		}
	}
}
