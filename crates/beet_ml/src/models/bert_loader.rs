use crate::prelude::*;
use bevy::asset::io::Reader;
use bevy::asset::AssetLoader;
use bevy::asset::AsyncReadExt;
use bevy::asset::LoadContext;
use bevy::utils::ConditionalSendFuture;

#[derive(Default)]
pub struct BertLoader;

impl AssetLoader for BertLoader {
	type Asset = Bert;
	type Settings = ();
	type Error = anyhow::Error;

	fn load<'a>(
		&'a self,
		reader: &'a mut Reader,
		_settings: &'a Self::Settings,
		_load_context: &'a mut LoadContext,
	) -> impl ConditionalSendFuture
	       + futures::Future<
		Output = Result<
			<Self as AssetLoader>::Asset,
			<Self as AssetLoader>::Error,
		>,
	> {
		Box::pin(async move {
			let mut bytes = Vec::new();
			reader.read_to_end(&mut bytes).await?;
			let config = ron::de::from_bytes::<BertConfig>(&bytes)?;
			let bert = Bert::new(config).await?;

			log::info!("bert loaded");

			Ok(bert)
		})
	}
}
