use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;


/// A unique identifier for a stat,
/// when defined by [`StatMap::from_sim_descriptor`] this is the index in the [`SimDescriptor`].
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
pub struct StatId(pub usize);


#[derive(Debug, Default, Clone, PartialEq, Resource)]
pub struct StatMap {
	pub map: HashMap<StatId, StatDescriptor>,
	next_id: usize,
}

impl std::ops::Deref for StatMap {
	type Target = HashMap<StatId, StatDescriptor>;
	fn deref(&self) -> &Self::Target { &self.map }
}

impl std::ops::DerefMut for StatMap {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.map }
}

impl StatMap {
	pub fn from_sim_descriptor(sim_descriptor: &SimDescriptor) -> Self {
		let mut map = Self::default();
		for stat in &sim_descriptor.stats {
			map.add_stat(stat.clone());
		}
		map
	}


	pub fn add_stat(&mut self, stat: StatDescriptor) -> StatId {
		let id = StatId(self.next_id);
		self.next_id += 1;
		self.map.insert(id, stat);
		id
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut sim_descriptor = SimDescriptor::default();
		let stat = StatDescriptor {
			name: "Health".to_string(),
			description: "The health of the entity".to_string(),
			emoji_hexcode: "❤️".to_string(),
			type_id: std::any::TypeId::of::<f32>(),
		};

		sim_descriptor.stats.push(stat.clone());

		let mut stat_map = StatMap::from_sim_descriptor(&sim_descriptor);

		expect(stat_map.map.len()).to_be(1)?;
		expect(stat_map.map.get(&StatId(0)).unwrap()).to_be(&stat)?;

		stat_map.add_stat(stat.clone());
		expect(stat_map.map.len()).to_be(2)?;
		expect(stat_map.map.get(&StatId(1)).unwrap()).to_be(&stat)?;


		Ok(())
	}
}
