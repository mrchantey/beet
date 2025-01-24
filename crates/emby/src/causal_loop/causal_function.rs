use serde::Deserialize;
use serde::Serialize;



/// Represents different types of causal relationships between variables in the system.
/// Each variant defines a specific mathematical relationship that determines how one
/// variable affects another over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CausalFunction {
	/// Represents linear growth where the target variable increases at a constant rate
	/// proportional to the source variable's value.
	///
	/// Formula: new_value = current_value + (rate * source_value * time_step)
	///
	/// Parameters:
	/// - rate: The growth rate coefficient (e.g., 0.1 means 10% growth per time step)
	///
	/// Example usage:
	/// ```rust
	/// # use emby::prelude::*;
	/// // Population grows linearly based on available resources
	/// CausalFunction::LinearGrowth { rate: 0.1 }
	/// // If resources = 1000, and current population = 500, with time_step = 1
	/// // New population = 500 + (0.1 * 1000 * 1) = 600
	/// ```
	LinearGrowth { rate: f32 },

	/// Represents exponential growth where the target variable grows by a percentage
	/// of its current value, modified by the time step.
	///
	/// Formula: new_value = current_value * (1 + rate * time_step)
	///
	/// Parameters:
	/// - rate: The exponential growth rate (e.g., 0.05 means 5% compound growth per time step)
	///
	/// Example usage:
	/// ```rust
	/// # use emby::prelude::*;
	/// // Virus spread grows exponentially
	/// CausalFunction::ExponentialGrowth { rate: 0.05 }
	/// // If current infected = 100, with time_step = 1
	/// // New infected = 100 * (1 + 0.05 * 1) = 105
	/// ```
	ExponentialGrowth { rate: f32 },

	/// Represents linear decay where the target variable decreases at a constant rate
	/// proportional to the source variable's value.
	///
	/// Formula: new_value = current_value - (rate * source_value * time_step)
	///
	/// Parameters:
	/// - rate: The decay rate coefficient (e.g., 0.1 means 10% decay per time step)
	///
	/// Example usage:
	/// ```rust
	/// # use emby::prelude::*;
	/// // Resources decay based on population size
	/// CausalFunction::LinearDecay { rate: 0.05 }
	/// // If population = 1000, and current resources = 500, with time_step = 1
	/// // New resources = 500 - (0.05 * 1000 * 1) = 450
	/// ```
	LinearDecay { rate: f32 },

	/// Represents a proportional effect where the change in the target variable
	/// is proportional to both its current value and the source variable.
	///
	/// Formula: new_value = current_value * (1 + factor * source_value * time_step)
	///
	/// Parameters:
	/// - factor: The proportionality factor that determines the strength of the effect
	///
	/// Example usage:
	/// ```rust
	/// // Economic growth affected by education level
	/// CausalFunction::ProportionalEffect { factor: 0.02 }
	/// // If education_level = 0.8, and current_gdp = 1000, with time_step = 1
	/// // New GDP = 1000 * (1 + 0.02 * 0.8 * 1) = 1016
	/// ```
	ProportionalEffect { factor: f32 },

	/// Represents a threshold-based effect where the target variable changes
	/// differently based on whether the source variable is above or below a threshold.
	///
	/// Formula:
	/// If source_value > threshold: new_value = current_value + above_value * time_step
	/// If source_value ≤ threshold: new_value = current_value + below_value * time_step
	///
	/// Parameters:
	/// - threshold: The value at which the behavior changes
	/// - above_value: The change rate when source is above threshold
	/// - below_value: The change rate when source is below threshold
	///
	/// Example usage:
	/// ```rust
	/// # use emby::prelude::*;
	/// // Stress level changes based on work hours threshold
	/// CausalFunction::ThresholdEffect {
	///     threshold: 40.0,  // 40 hours per week
	///     above_value: 2.0, // Fast stress increase above threshold
	///     below_value: -1.0 // Slow stress decrease below threshold
	/// }
	/// // If work_hours = 45, and current_stress = 50, with time_step = 1
	/// // New stress = 50 + 2.0 * 1 = 52
	/// ```
	ThresholdEffect {
		threshold: f32,
		above_value: f32,
		below_value: f32,
	},

	/// Represents an oscillating effect where the target variable changes
	/// in a sinusoidal pattern over time.
	///
	/// Formula: new_value = current_value + amplitude * sin(frequency * time_step * π)
	///
	/// Parameters:
	/// - amplitude: The maximum change in either direction
	/// - frequency: How quickly the oscillation occurs (cycles per time step)
	///
	/// Example usage:
	/// ```rust
	/// # use emby::prelude::*;
	///
	/// // Temperature variations throughout the day
	/// CausalFunction::Oscillation {
	///     amplitude: 5.0,   // 5 degree variation
	///     frequency: 0.25   // Complete cycle every 4 time steps
	/// }
	/// // If current_temp = 20, with time_step = 1
	/// // New temp = 20 + 5.0 * sin(0.25 * 1 * π) ≈ 23.5
	/// ```
	Oscillation { amplitude: f32, frequency: f32 },
}

// Implementation showing how each function type is applied
impl CausalFunction {
	/// Applies the causal function to calculate the new value based on the current value,
	/// source value, and time step.
	///
	/// Parameters:
	/// - current_value: The current value of the target variable
	/// - source_value: The current value of the source variable affecting the target
	/// - time_step: The size of the time step for this update
	///
	/// Returns:
	/// The new value after applying the causal function
	///
	/// Example:
	/// ```rust
	/// # use emby::prelude::*;
	/// let function = CausalFunction::LinearGrowth { rate: 0.1 };
	/// let new_value = function.apply(100.0, 50.0, 1.0);
	/// ```
	pub fn apply(
		&self,
		current_value: f32,
		source_value: f32,
		time_step: f32,
	) -> f32 {
		match self {
			CausalFunction::LinearGrowth { rate } => {
				current_value + (rate * source_value * time_step)
			}
			CausalFunction::ExponentialGrowth { rate } => {
				current_value * (1.0 + rate * time_step)
			}
			CausalFunction::LinearDecay { rate } => {
				current_value - (rate * source_value * time_step)
			}
			CausalFunction::ProportionalEffect { factor } => {
				current_value * (1.0 + factor * source_value * time_step)
			}
			CausalFunction::ThresholdEffect {
				threshold,
				above_value,
				below_value,
			} => {
				if source_value > *threshold {
					current_value + above_value * time_step
				} else {
					current_value + below_value * time_step
				}
			}
			CausalFunction::Oscillation {
				amplitude,
				frequency,
			} => {
				current_value
					+ amplitude
						* (frequency * time_step * std::f32::consts::PI).sin()
			}
		}
	}

	/// Returns a human-readable description of the function and its parameters
	pub fn get_description(&self) -> String {
		match self {
			CausalFunction::LinearGrowth { rate } => {
				format!("Linear growth with rate {:.2} per time step", rate)
			}
			CausalFunction::ExponentialGrowth { rate } => {
				format!(
					"Exponential growth with rate {:.2}% per time step",
					rate * 100.0
				)
			}
			CausalFunction::LinearDecay { rate } => {
				format!("Linear decay with rate {:.2} per time step", rate)
			}
			CausalFunction::ProportionalEffect { factor } => {
				format!("Proportional effect with factor {:.2}", factor)
			}
			CausalFunction::ThresholdEffect {
				threshold,
				above_value,
				below_value,
			} => {
				format!(
					"Threshold effect at {:.2}: {:.2} above, {:.2} below",
					threshold, above_value, below_value
				)
			}
			CausalFunction::Oscillation {
				amplitude,
				frequency,
			} => {
				format!(
					"Oscillation with amplitude {:.2} and frequency {:.2}",
					amplitude, frequency
				)
			}
		}
	}
}
