use beet_ml::prelude::*;
use bevy::scene::ron;
use std::fs::File;
use std::fs::{
	self,
};
use std::io::Write;
use sweet::prelude::*;

fn main() -> Result {
	let map = FrozenLakeMap::default_four_by_four();
	let initial_state = map.agent_position();
	let env = QTableEnv::new(map.transition_outcomes());
	let params = QLearnParams::default();

	let mut trainer = QTableTrainer::<FrozenLakeQTableSession>::new(
		env.clone(),
		QTable::default(),
		params,
		initial_state,
	);
	trainer.train(&mut RandomSource::default().0);
	let eval = trainer.evaluate();
	assert_eq!(eval.mean, 1.);
	assert_eq!(eval.std, 0.);
	assert_eq!(eval.total_steps, 600);
	// println!("Model trained\nMean: {}, Std: {}", eval.mean, eval.std);

	let table = trainer.table;
	let text = ron::ser::to_string_pretty(&table, Default::default())?;
	fs::create_dir_all("assets/ml")?;
	File::create("assets/ml/frozen_lake_qtable.ron")
		.and_then(|mut file| file.write(text.as_bytes()))?;
	// save table to ron file
	Ok(())
}
