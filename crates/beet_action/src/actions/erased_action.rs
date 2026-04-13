use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Component)]
pub struct ErasedAction {
	inner: Action<(MediaBytes<'static>, Vec<MediaType>), MediaBytes<'static>>,
}

impl ErasedAction {
	pub fn new<In, Out>() -> Self
	where
		In: 'static + Send + Sync + DeserializeOwned,
		Out: 'static + Send + Sync + Serialize,
	{
		Self {
			inner: Action::<
				(MediaBytes<'static>, Vec<MediaType>),
				MediaBytes<'static>,
			>::new_async(async |cx| -> Result<MediaBytes<'static>> {
				let (input, accepts) = cx.input;
				let input: In = input.deserialize()?;
				let output: Out = cx.caller.call(input).await?;
				let output = MediaType::serialize_accepts(&accepts, &output)?;
				Ok(output)
			}),
		}
	}
	pub async fn call(
		&self,
		entity: AsyncEntity,
		input: MediaBytes<'static>,
		accepts: Vec<MediaType>,
	) -> Result<MediaBytes<'static>> {
		entity
			.call_detached(self.inner.clone(), (input, accepts))
			.await
	}
}
