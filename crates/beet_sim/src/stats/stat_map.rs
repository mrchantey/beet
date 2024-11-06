use super::StatDescriptor;
use bevy::prelude::*;
use bevy::utils::HashMap;




#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatId(pub usize);


#[derive(Debug, Default, Clone, PartialEq, Resource)]
pub struct StatMap {
	pub map: HashMap<StatId, StatDescriptor>,
	next_id: usize,
}

impl StatMap {
	pub fn add_stat(&mut self, stat: StatDescriptor) -> StatId {
		let id = StatId(self.next_id);
		self.next_id += 1;
		self.map.insert(id, stat);
		id
	}
}
