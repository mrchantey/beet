use crate::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;


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
	pub fn with_stat(mut self, stat: StatDescriptor) -> Self {
		self.add_stat(stat);
		self
	}


	pub fn add_stat(&mut self, stat: StatDescriptor) -> StatId {
		let id = StatId(self.next_id);
		self.next_id += 1;
		self.map.insert(id, stat);
		id
	}
	/// Get the first stat with the given name.
	pub fn get_by_name(
		&self,
		name: &str,
	) -> Option<(&StatId, &StatDescriptor)> {
		self.map.iter().find(|(_, stat)| stat.name == name)
	}

	pub fn get_id_by_name(&self, name: &str) -> Option<StatId> {
		self.get_by_name(name).map(|(id, _)| *id)
	}
	pub fn get_default_by_name(&self, name: &str) -> Option<StatValue> {
		self.get_by_name(name).map(|(_, desc)| desc.default_value)
	}

	#[cfg(test)]
	pub fn default_with_test_stats() -> Self {
		let mut stat_map = StatMap::default();
		stat_map.add_stat(StatDescriptor {
			name: "Health".to_string(),
			description: "The health of the agent".to_string(),
			emoji_hexcode: "2764".to_string(),
			global_range: StatValue::range(0.0..1.),
			default_value: StatValue(1.),
		});
		stat_map.add_stat(StatDescriptor {
			name: "Pleasantness".to_string(),
			description: "How groovy the agent is feeling".to_string(),
			emoji_hexcode: "1F600".to_string(),
			global_range: StatValue::range(-5.0..5.0),
			default_value: StatValue(0.),
		});
		stat_map
	}
	#[cfg(test)]
	pub const TEST_HEALTH_ID: StatId = StatId(0);
	#[cfg(test)]
	pub const TEST_PLEASENTNESS_ID: StatId = StatId(1);
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut sim_descriptor = SimDescriptor::default();
		let stat = StatDescriptor {
			name: "Health".to_string(),
			description: "The health of the entity".to_string(),
			emoji_hexcode: "❤️".to_string(),
			global_range: StatValue::range(0.0..1.),
			default_value: StatValue(1.),
		};

		sim_descriptor.stats.push(stat.clone());

		let mut stat_map = StatMap::from_sim_descriptor(&sim_descriptor);

		stat_map.map.len().xpect().to_be(1);
		stat_map.map.get(&StatId(0)).unwrap().xpect().to_be(&stat);

		stat_map.add_stat(stat.clone());
		stat_map.map.len().xpect().to_be(2);
		stat_map.map.get(&StatId(1)).unwrap().xpect().to_be(&stat);
	}
}
