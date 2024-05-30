use bevy::prelude::*;

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

impl Plugin for FrozenLakePlugin {
	fn build(&self, app: &mut App) {}
}





#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::math::UVec2;
	use frozen_lake::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let map = FrozenLakeMap::default_four_by_four();
		let actions = TranslateGrid::default();
		println!("{:?}", map.shape());
		println!("{:?}", map.sample());
		println!("{:?}", actions.shape());
		println!("{:?}", actions.sample());

		let table = QTable::<
			{ FrozenLakeMap::<16>::LEN },
			{ TranslateGrid::LEN },
		>::default();

		println!("table: {:?}", table);

		let mut env = FrozenLakeEnv::new(map, true);

		let action = TranslateGridDirection::Left;
		let out = env.step(action);
		expect(out.new_pos).to_be(UVec2::new(0, 0))?;
		let action = TranslateGridDirection::Down;
		let out = env.step(action);
		expect(out.new_pos).to_be(UVec2::new(0, 1))?;


		Ok(())
	}
}
