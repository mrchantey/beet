pub type EmbyFactorId = usize;



pub struct EmbyScenario {
	pub name: String,
	pub description: String,
	pub factors: Vec<EmbyFactor>,
	pub systems: Vec<EmbySystem>,
}


pub struct EmbyFactor {
	pub name: String,
	pub id: usize,
	pub kind: EmbyFactorKind,
}


pub enum EmbyFactorKind {
	Discrete(i32),
	Continuous(f32),
}

pub struct EmbySystem {
	pub target_factor: EmbyFactorId,
}
