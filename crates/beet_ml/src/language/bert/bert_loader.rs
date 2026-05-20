use crate::language::bert::Bert;
use crate::language::bert::BertConfig;
use beet_core::prelude::*;
use bevy::asset::AssetLoader;
use bevy::asset::LoadContext;
use bevy::asset::io::Reader;
use bevy::tasks::ConditionalSendFuture;

/// `AssetLoader` for `.ron` files containing a [`BertConfig`].
///
/// Reads the config, then awaits [`Bert::new`] (which downloads the
/// underlying safetensors / tokenizer via [`fetch_bytes`]). Lets you
/// drop a config next to your other assets and `asset_server.load::<Bert>(...)`.
///
/// [`fetch_bytes`]: crate::fetch::fetch_bytes
#[derive(Default, TypePath)]
pub struct BertLoader;

impl AssetLoader for BertLoader {
	type Asset = Bert;
	type Settings = ();
	type Error = BevyError;

	fn load(
		&self,
		reader: &mut dyn Reader,
		_settings: &Self::Settings,
		_load_context: &mut LoadContext,
	) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>>
	{
		Box::pin(async move {
			let mut bytes = Vec::new();
			reader.read_to_end(&mut bytes).await?;
			let config: BertConfig = ron::de::from_bytes(&bytes)?;
			Bert::new(config).await
		})
	}

	fn extensions(&self) -> &[&str] { &["ron"] }
}
