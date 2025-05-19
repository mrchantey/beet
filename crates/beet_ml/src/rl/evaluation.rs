#[derive(Debug, Clone)]
pub struct Evaluation {
	pub total_steps: u128,
	pub mean: f32,
	pub std: f32,
}

impl Evaluation {
	pub fn new(rewards: Vec<f32>, total_steps: u128) -> Self {
		let mean = mean(&rewards).unwrap();
		let std = variance(&rewards).unwrap();
		Self {
			mean,
			std,
			total_steps,
		}
	}
}

fn mean(data: &[f32]) -> Option<f32> {
	let len = data.len();
	if len == 0 {
		return None;
	}
	Some(data.iter().sum::<f32>() / len as f32)
}


fn variance(data: &[f32]) -> Option<f32> {
	let len = data.len();
	if len < 2 {
		return None;
	}

	let mean = data.iter().sum::<f32>() / len as f32;
	let var = data.iter().map(|value| (value - mean).powi(2)).sum::<f32>()
		/ (len - 1) as f32;
	Some(var)
}
