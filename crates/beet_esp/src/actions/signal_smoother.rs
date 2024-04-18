use std::collections::VecDeque;



pub struct SignalSmoother {
	buffer: VecDeque<f32>,
	/// Should be proportional to the frequency of the input signal
	/// a good starting point is 5x delay, ie 10ms delay -> 50 buffer size
	buffer_size: usize,
}

impl Default for SignalSmoother {
	fn default() -> Self { Self::new(50) }
}

impl SignalSmoother {
	pub fn new(buffer_size: usize) -> Self {
		Self {
			buffer: VecDeque::with_capacity(buffer_size),
			buffer_size,
		}
	}

	pub fn add_and_smooth(&mut self, value: f32) -> f32 {
		self.buffer.push_back(value);
		if self.buffer.len() > self.buffer_size {
			self.buffer.pop_front();
		}
		self.buffer.iter().sum::<f32>() / self.buffer.len() as f32
	}
}
