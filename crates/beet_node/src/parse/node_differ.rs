use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::futures_lite::Stream;



pub trait NodeDiffer {
	fn diff(
		&mut self,
		entity: AsyncEntity,
		stream: impl 'static + Stream<Item = Result<impl AsRef<[u8]>>>,
	) -> impl Future<Output = Result>;
}


pub struct PlainTextParser {}

impl PlainTextParser {
	pub fn new() -> Self { Self {} }
}



impl NodeDiffer for PlainTextParser {
	fn diff(
		&mut self,
		entity: AsyncEntity,
		stream: impl 'static + Stream<Item = Result<impl AsRef<[u8]>>>,
	) -> impl Future<Output = Result> {
		let mut stream = stream.boxed_local();
		// just collect the bytes instead of appending to an existing Value
		let mut bytes = Vec::new();
		async move {
			while let Some(result) = stream.next().await {
				let data = result?;
				bytes.extend_from_slice(data.as_ref());
				let string = String::from_utf8_lossy(&bytes);
				entity.insert_then(Value::new(string)).await;
			}
			Ok(())
		}
	}
}
