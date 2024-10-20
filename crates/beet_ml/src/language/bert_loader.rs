use crate::prelude::*;
use bevy::asset::io::Reader;
use bevy::asset::AssetLoader;
use bevy::asset::LoadContext;
use bevy::utils::ConditionalSendFuture;

#[derive(Default)]
pub struct BertLoader;

impl AssetLoader for BertLoader {
	type Asset = Bert;
	type Settings = ();
	type Error = anyhow::Error;

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
	use anyhow::Result;
	use beet_flow::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();

		app.add_plugins((TaskPoolPlugin::default(), workspace_asset_plugin()))
			.init_asset::<Bert>()
			.init_asset_loader::<BertLoader>();

		block_on_asset_load::<Bert>(&mut app, "ml/default-bert.ron")?;

		expect(true).to_be_true()?;

		Ok(())
	}
}
