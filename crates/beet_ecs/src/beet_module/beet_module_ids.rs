use std::any::TypeId;


/// All ids registered by this module and any sub modules
pub struct BeetModuleIds {
	pub action_ids: Vec<TypeId>,
	pub component_ids: Vec<TypeId>,
	pub bundle_ids: Vec<TypeId>,
}
impl BeetModuleIds {
	pub fn extend(&mut self, other: Self) {
		self.action_ids.extend(other.action_ids);
		self.component_ids.extend(other.component_ids);
		self.bundle_ids.extend(other.bundle_ids);
	}
}
