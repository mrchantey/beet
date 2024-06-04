use crate::prelude::*;
use bevy::asset::io::Reader;
use bevy::asset::AssetLoader;
use bevy::asset::AsyncReadExt;
use bevy::asset::LoadContext;
use bevy::prelude::*;
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

			Ok(bert)
		})
	}
}

#[cfg(feature = "beet_net")]
pub fn ready_on_bert_load(
	mut asset_events: EventReader<AssetEvent<Bert>>,
	mut ready_events: EventWriter<beet_net::replication::AppReady>,
) {
	for ev in asset_events.read() {
		match ev {
			AssetEvent::LoadedWithDependencies { id: _ } => {
				ready_events.send(beet_net::replication::AppReady);
			}
			_ => {}
		}
	}
}

pub fn block_on_asset_load<'a, A: Asset>(app: &'a mut App, path: &'static str) {
	let handle = app
		.world_mut()
		.resource_mut::<AssetServer>()
		.load::<A>(path)
		.clone();
	loop {
		match app
			.world_mut()
			.resource_mut::<AssetServer>()
			.load_state(handle.id())
		{
			bevy::asset::LoadState::Loaded => return,
			_ => {}
		}
		app.update();
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();

		app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()))
			.init_asset::<Bert>()
			.init_asset_loader::<BertLoader>();

		block_on_asset_load::<Bert>(&mut app, "default-bert.ron");

		expect(true).to_be_true()?;

		Ok(())
	}
}
