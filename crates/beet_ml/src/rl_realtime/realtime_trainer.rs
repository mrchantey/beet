use crate::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;


pub struct EpisodeParams<'a, 'w, 's> {
	pub commands: &'a mut Commands<'w, 's>,
	pub episode: u32,
	pub trainer_entity: Entity,
	pub params: QLearnParams,
}


pub type EpisodeFunc = fn(EpisodeParams);

#[derive(Component)]
pub struct RealtimeTrainer {
	episode_start: EpisodeFunc,
	#[allow(unused)]
	episode_end: EpisodeFunc,
	params: QLearnParams,
	filename: Option<Cow<'static, str>>,
	episode: u32,
}


impl RealtimeTrainer {
	pub fn new(
		episode_start: EpisodeFunc,
		episode_end: EpisodeFunc,
		params: QLearnParams,
	) -> Self {
		Self {
			episode_start,
			episode_end,
			params,
			filename: None,
			episode: 0,
		}
	}
	pub fn with_outfile(
		mut self,
		filename: impl Into<Cow<'static, str>>,
	) -> Self {
		self.filename = Some(filename.into());
		self
	}

	pub fn episode_params<'a, 'w, 's>(
		&self,
		trainer_entity: Entity,
		commands: &'a mut Commands<'w, 's>,
	) -> EpisodeParams<'a, 'w, 's> {
		EpisodeParams {
			commands,
			trainer_entity,
			episode: self.episode,
			params: self.params.clone(),
		}
	}
}


pub fn start_realtime_trainer(
	mut commands: Commands,
	mut trainers: Query<(Entity, &mut RealtimeTrainer), Added<RealtimeTrainer>>,
) {
	for (entity, trainer) in trainers.iter_mut() {
		let episode_params = trainer.episode_params(entity, &mut commands);
		(trainer.episode_start)(episode_params);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	// use sweet::*;

	fn episode_start(_params: EpisodeParams) {}
	fn episode_end(_params: EpisodeParams) {}

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();

		app.world_mut().spawn(RealtimeTrainer::new(
			episode_start,
			episode_end,
			QLearnParams::new(),
		));


		Ok(())
	}
}
