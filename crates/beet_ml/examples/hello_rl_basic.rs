//! Trains a Q-table on the 4x4 frozen lake and writes it to
//! `assets/ml/frozen_lake_qtable.ron`.
use beet_core::prelude::*;
use beet_ml::prelude::*;

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

	let text = ron::ser::to_string_pretty(&trainer.table, Default::default())?;
	fs_ext::write("assets/ml/frozen_lake_qtable.ron", text)?;
	Ok(())
}
