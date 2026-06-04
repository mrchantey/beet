use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Component)]
pub struct ErasedAction {
	inner: Action<(MediaBytes, Vec<MediaType>), MediaBytes>,
}

impl ErasedAction {
	pub fn new<In, Out>() -> Self
	where
		In: 'static + Send + Sync + DeserializeOwned,
		Out: 'static + Send + Sync + Serialize,
	{
		Self {
			inner:
				Action::<(MediaBytes, Vec<MediaType>), MediaBytes>::new_async(
					async |cx| -> Result<MediaBytes> {
						let (input, accepts) = cx.input;
						let input: In = input.deserialize()?;
						let output: Out = cx.caller.call(input).await?;
						let output =
							MediaType::serialize_accepts(&accepts, &output)?;
						Ok(output)
					},
				),
		}
	}
	pub async fn call(
		&self,
		entity: AsyncEntity,
		input: MediaBytes,
		accepts: Vec<MediaType>,
	) -> Result<MediaBytes> {
		entity
			.call_detached(self.inner.clone(), (input, accepts))
			.await
	}
}
