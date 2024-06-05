// use bevy::prelude::*;

/**
Implementation of the OpenAI Gym Frozen Lake environment.
https://github.com/openai/gym/blob/master/gym/envs/toy_text/frozen_lake.py

Frozen lake involves crossing a frozen lake from Start(S) to Goal(G) without falling into any Holes(H)
by walking over the Frozen(F) lake.
The agent may not always move in the intended direction due to the slippery nature of the frozen lake.

### Action Space
The agent takes a 1-element vector for actions.
The action space is `(dir)`, where `dir` decides direction to move in which can be:

- 0: LEFT
- 1: DOWN
- 2: RIGHT
- 3: UP

### Observation Space
The observation is a value representing the agent's current position as
current_row * nrows + current_col (where both the row and col start at 0).
For example, the goal position in the 4x4 map can be calculated as follows: 3 * 4 + 3 = 15.
The number of possible observations is dependent on the size of the map.
For example, the 4x4 map has 16 possible observations.

### Rewards

Reward schedule:
- Reach goal(G): +1
- Reach hole(H): 0
- Reach frozen(F): 0
**/
pub struct FrozenLakePlugin;

// impl Plugin for FrozenLakePlugin {
// 	fn build(&self, app: &mut App) {}
// }





#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use rand::rngs::StdRng;
	use rand::SeedableRng;
	use std::time::Duration;
	use std::time::Instant;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let map = FrozenLakeMap::default_four_by_four();
		let mut table = QTable::default();

		let slippery_rng = StdRng::seed_from_u64(0);
		let mut policy_rng = StdRng::seed_from_u64(10);
		let new_env = move || {
			FrozenLakeEnv::new(map, false)
				.with_slippery_rng(slippery_rng.clone())
		};

		let mut trainer = QTableTrainer::new();
		let now = Instant::now();
		trainer.train(&mut table, &new_env, &mut policy_rng);
		let elapsed = now.elapsed();
		println!("\nTrained in: {:.2?} seconds\n", elapsed.as_secs_f32());
		// println!("trained table: {:?}", table);
		expect(elapsed).to_be_greater_than(Duration::from_millis(30))?;
		expect(elapsed).to_be_less_than(Duration::from_millis(70))?;

		let evaluation = trainer.evaluate(&table, &new_env);
		println!("{evaluation:?}\n");

		expect(evaluation.mean).to_be_greater_than(0.99)?;
		expect(evaluation.std).to_be_close_to(0.00)?;
		expect(evaluation.total_steps).to_be(600)?;

		Ok(())
	}
}
