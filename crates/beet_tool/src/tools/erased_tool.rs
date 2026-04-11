use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;

#[derive(Component)]
pub struct ErasedTool {
	handler: Arc<
		dyn 'static
			+ Send
			+ Sync
			+ Fn(
				AsyncEntity,
				MediaBytes<'static>,
				Vec<MediaType>,
			) -> SendBoxedFuture<Result<MediaBytes<'static>>>,
	>,
}

impl ErasedTool {
	pub fn new<In, Out>() -> Self
	where
		In: 'static + Send + Sync + DeserializeOwned,
		Out: 'static + Send + Sync + Serialize,
	{
		Self {
			handler: Arc::new(|entity, input, accepts| {
				Box::pin(async move {
					let input: In = input.deserialize()?;
					let output: Out = entity.call(input).await?;
					let output =
						MediaType::serialize_accepts(&accepts, &output)?;
					Ok(output)
				})
			}),
		}
	}
	pub fn call(
		&self,
		entity: AsyncEntity,
		input: MediaBytes<'static>,
		accepts: Vec<MediaType>,
	) -> SendBoxedFuture<Result<MediaBytes<'static>>> {
		(self.handler)(entity, input, accepts)
	}
}
