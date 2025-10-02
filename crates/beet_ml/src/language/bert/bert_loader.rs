use crate::prelude::*;
use beet_core::prelude::*;
use bevy::asset::AssetLoader;
use bevy::asset::LoadContext;
use bevy::asset::io::Reader;
use bevy::tasks::ConditionalSendFuture;

#[derive(Default)]
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
			let config = ron::de::from_bytes::<BertConfig>(&bytes)?;
			let bert = Bert::new(config).await?;

			Ok(bert)
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	// possibly flaky tests here, getting occasional 403 on tokenizer.json
	fn works() {
		let mut app = App::new();

		app.add_plugins((TaskPoolPlugin::default(), workspace_asset_plugin()))
			.init_asset::<Bert>()
			.init_asset_loader::<BertLoader>();

		block_on_asset_load::<Bert>(&mut app, "ml/default-bert.ron").unwrap();
	}
}
