use crate::prelude::*;
use bevy::asset::io::Reader;
use bevy::asset::AssetLoader;
use bevy::asset::LoadContext;
use bevy::utils::ConditionalSendFuture;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

#[derive(Default)]
pub struct QTableLoader<State: StateSpace, Action: ActionSpace> {
	phantom: PhantomData<(State, Action)>,
}

impl<
		State: StateSpace + DeserializeOwned,
		Action: ActionSpace + DeserializeOwned,
	> AssetLoader for QTableLoader<State, Action>
{
	type Asset = QTable<State, Action>;
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
			let table = bevy::scene::ron::de::from_bytes::<
				QTable<State, Action>,
			>(&bytes)?;
			Ok(table)
		})
	}
}
