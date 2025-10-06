//! Enhanced moon phase calculator with precise timing predictions
//!
//! Based on the algorithm from 'Astronomical Computing' column of Sky & Telescope, by Bradley E. Schaefer.
//! This version adds functionality to find exact times for upcoming lunar phases.
//!
//! ## Features
//! - Current moon phase calculation
//! - Precise timing for next major phases (New, First Quarter, Full, Last Quarter)
//! - Custom phase targeting (any phase value 0.0-1.0)
//! - Multiple upcoming phases listing
//! - Accurate date/time formatting
//!
//! ## Usage
//! ```
//! let now = SystemTime::now();
//! let next_full = next_full_moon(now);
//! let next_new = next_new_moon(now);
//! let custom_phase = next_phase(now, 0.375); // Waxing Gibbous
//! ```

use std::f64::consts::TAU;
use std::time::SystemTime;

const MOON_SYNODIC_PERIOD: f64 = 29.530588853; // Period of moon cycle in days.
const MOON_SYNODIC_OFFSET: f64 = 2451550.26; // Reference cycle offset in days.
const MOON_DISTANCE_PERIOD: f64 = 27.55454988; // Period of distance oscillation
const MOON_DISTANCE_OFFSET: f64 = 2451562.2;
const MOON_LATITUDE_PERIOD: f64 = 27.212220817; // Latitude oscillation
const MOON_LATITUDE_OFFSET: f64 = 2451565.2;
const MOON_LONGITUDE_PERIOD: f64 = 27.321582241; // Longitude oscillation
const MOON_LONGITUDE_OFFSET: f64 = 2451555.8;

fn main() {
	let now = SystemTime::now();
	let phase = MoonPhase::new(now);
	println!(
		"Current moon phase: {} ({:.1}% illuminated, {:.1} days old)",
		phase.phase_name,
		phase.fraction.abs() * 100.0,
		phase.age
	);

	println!("\nNext major phases:");
	let new_moon = next_new_moon(now);
	let first_quarter = next_first_quarter(now);
	let full_moon = next_full_moon(now);
	let last_quarter = next_last_quarter(now);

	println!(
		"New Moon: {} ({})",
		format_time(new_moon),
		time_until(now, new_moon)
	);
	println!(
		"First Quarter: {} ({})",
		format_time(first_quarter),
		time_until(now, first_quarter)
	);
	println!(
		"Full Moon: {} ({})",
		format_time(full_moon),
		time_until(now, full_moon)
	);
	println!(
		"Last Quarter: {} ({})",
		format_time(last_quarter),
		time_until(now, last_quarter)
	);

	println!("\nNext 8 phases:");
	let upcoming = next_phases(now, 8);
	for (phase_name, time) in upcoming {
		println!(
			"{}: {} ({})",
			phase_name,
			format_time(time),
			time_until(now, time)
		);
	}

	// Example: find next waxing gibbous (phase ~0.375)
	println!("\nCustom phase examples:");
	let waxing_gibbous = next_waxing_gibbous(now);
	println!(
		"Next Waxing Gibbous: {} ({})",
		format_time(waxing_gibbous),
		time_until(now, waxing_gibbous)
	);

	let waning_crescent = next_waning_crescent(now);
	println!(
		"Next Waning Crescent: {} ({})",
		format_time(waning_crescent),
		time_until(now, waning_crescent)
	);

	// Custom phase value
	let custom = next_phase(now, 0.6);
	println!(
		"Next phase 0.6 ({}): {} ({})",
		phase_name(0.6),
		format_time(custom),
		time_until(now, custom)
	);
}


// Names of lunar phases
const PHASE_NAMES: &[&str] = &[
	"New",
	"Waxing Crescent",
	"First Quarter",
	"Waxing Gibbous",
	"Full",
	"Waning Gibbous",
	"Last Quarter",
	"Waning Crescent",
];
// Names of Zodiac constellations
const ZODIAC_NAMES: [&str; 12] = [
	"Pisces",
	"Aries",
	"Taurus",
	"Gemini",
	"Cancer",
	"Leo",
	"Virgo",
	"Libra",
	"Scorpio",
	"Sagittarius",
	"Capricorn",
	"Aquarius",
];
// Ecliptic angles of Zodiac constellations
const ZODIAC_ANGLES: [f64; 12] = [
	33.18, 51.16, 93.44, 119.48, 135.30, 173.34, 224.17, 242.57, 271.26,
	302.49, 311.72, 348.58,
];

#[derive(Debug, Copy, Clone)]
pub struct MoonPhase {
	pub j_date: f64,
	pub phase: f64,                // 0 - 1, 0.5 = full
	pub age: f64,                  // Age in days of current cycle
	pub fraction: f64,             // Fraction of illuminated disk
	pub distance: f64,             // Moon distance in earth radii
	pub latitude: f64,             // Moon ecliptic latitude
	pub longitude: f64,            // Moon ecliptic longitude
	pub phase_name: &'static str,  // New, Full, etc.
	pub zodiac_name: &'static str, // Constellation
}

fn julian_date(time: SystemTime) -> f64 {
	let secs = match time.duration_since(SystemTime::UNIX_EPOCH) {
		Ok(duration) => duration.as_secs_f64(),
		Err(earlier) => -1. * earlier.duration().as_secs_f64(),
	};
	secs / 86400. + 2440587.5
}

impl MoonPhase {
	pub fn new(time: SystemTime) -> Self {
		let j_date = julian_date(time);

		// Calculate illumination (synodic) phase.
		// From number of days since new moon on Julian date MOON_SYNODIC_OFFSET
		// (1815UTC January 6, 2000), determine remainder of incomplete cycle.
		let phase =
			((j_date - MOON_SYNODIC_OFFSET) / MOON_SYNODIC_PERIOD).fract();
		// Calculate age and illuination fraction.
		let age = phase * MOON_SYNODIC_PERIOD;
		let fraction = (1. - (std::f64::consts::TAU * phase)).cos() / 2.;
		let phase_name = PHASE_NAMES[(phase * 8.).round() as usize % 8];
		// Calculate distance fro anoalistic phase.
		let distance_phase =
			((j_date - MOON_DISTANCE_OFFSET) / MOON_DISTANCE_PERIOD).fract();
		let distance_phase_tau = TAU * distance_phase;
		let phase_tau = 2. * TAU * phase;
		let phase_distance_tau_difference = phase_tau - distance_phase_tau;
		let distance = 60.4
			- 3.3 * distance_phase_tau.cos()
			- 0.6 * (phase_distance_tau_difference).cos()
			- 0.5 * (phase_tau).cos();

		// Calculate ecliptic latitude from nodal (draconic) phase.
		let lat_phase =
			((j_date - MOON_LATITUDE_OFFSET) / MOON_LATITUDE_PERIOD).fract();
		let latitude = 5.1 * (TAU * lat_phase).sin();

		// Calculate ecliptic longitude ffrom sidereal motion.
		let long_phase =
			((j_date - MOON_LONGITUDE_OFFSET) / MOON_LONGITUDE_PERIOD).fract();
		let longitude = (360. * long_phase
			+ 6.3 * (distance_phase_tau).sin()
			+ 1.3 * (phase_distance_tau_difference).sin()
			+ 0.7 * (phase_tau).sin())
			% 360.;

		let zodiac_name = ZODIAC_ANGLES
			.iter()
			.zip(ZODIAC_NAMES.iter())
			.find_map(|(angle, name)| {
				if longitude < *angle {
					Some(*name)
				} else {
					None
				}
			})
			.unwrap_or_else(|| ZODIAC_NAMES[0]);
		MoonPhase {
			j_date,
			phase,
			age,
			fraction,
			distance,
			latitude,
			longitude,
			phase_name,
			zodiac_name,
		}
	}
}

/// Find the next occurrence of a specific moon phase
fn next_phase_time(current_time: SystemTime, target_phase: f64) -> SystemTime {
	let current_phase = MoonPhase::new(current_time);

	// Calculate how far we need to advance to reach target phase
	let mut phase_diff = target_phase - current_phase.phase;
	if phase_diff <= 0.0 {
		phase_diff += 1.0; // Go to next cycle
	}

	// Estimate days to target phase
	let days_to_target = phase_diff * MOON_SYNODIC_PERIOD;
	let target_time = current_time
		+ std::time::Duration::from_secs_f64(days_to_target * 86400.0);

	// Fine-tune with binary search
	binary_search_phase(target_time, target_phase)
}

/// Binary search to find precise phase timing
fn binary_search_phase(
	approximate_time: SystemTime,
	target_phase: f64,
) -> SystemTime {
	let mut low_time = approximate_time - std::time::Duration::from_secs(86400); // -1 day
	let mut high_time =
		approximate_time + std::time::Duration::from_secs(86400); // +1 day

	// Binary search for 15 iterations (gives us ~3 second precision)
	for _ in 0..15 {
		let low_secs = low_time
			.duration_since(SystemTime::UNIX_EPOCH)
			.unwrap()
			.as_secs_f64();
		let high_secs = high_time
			.duration_since(SystemTime::UNIX_EPOCH)
			.unwrap()
			.as_secs_f64();
		let mid_secs = (low_secs + high_secs) / 2.0;
		let mid_time = SystemTime::UNIX_EPOCH
			+ std::time::Duration::from_secs_f64(mid_secs);

		let phase_info = MoonPhase::new(mid_time);
		let mut diff = target_phase - phase_info.phase;

		// Handle phase wrap-around
		if diff < -0.5 {
			diff += 1.0;
		} else if diff > 0.5 {
			diff -= 1.0;
		}

		if diff.abs() < 0.0001 {
			return mid_time;
		}

		if diff > 0.0 {
			low_time = mid_time;
		} else {
			high_time = mid_time;
		}
	}

	let low_secs = low_time
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap()
		.as_secs_f64();
	let high_secs = high_time
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap()
		.as_secs_f64();
	let mid_secs = (low_secs + high_secs) / 2.0;
	SystemTime::UNIX_EPOCH + std::time::Duration::from_secs_f64(mid_secs)
}

/// Find the next new moon (phase = 0.0)
pub fn next_new_moon(current_time: SystemTime) -> SystemTime {
	next_phase_time(current_time, 0.0)
}

/// Find the next first quarter moon (phase = 0.25)
pub fn next_first_quarter(current_time: SystemTime) -> SystemTime {
	next_phase_time(current_time, 0.25)
}

/// Find the next full moon (phase = 0.5)
pub fn next_full_moon(current_time: SystemTime) -> SystemTime {
	next_phase_time(current_time, 0.5)
}

/// Find the next last quarter moon (phase = 0.75)
pub fn next_last_quarter(current_time: SystemTime) -> SystemTime {
	next_phase_time(current_time, 0.75)
}

/// Find the next occurrence of any specific phase (0.0 to 1.0)
///
/// Phase values:
/// - 0.0 = New Moon
/// - 0.125 = Waxing Crescent
/// - 0.25 = First Quarter
/// - 0.375 = Waxing Gibbous
/// - 0.5 = Full Moon
/// - 0.625 = Waning Gibbous
/// - 0.75 = Last Quarter
/// - 0.875 = Waning Crescent
pub fn next_phase(current_time: SystemTime, target_phase: f64) -> SystemTime {
	next_phase_time(current_time, target_phase.clamp(0.0, 1.0))
}

/// Get the next N lunar phases in chronological order
pub fn next_phases(
	current_time: SystemTime,
	count: usize,
) -> Vec<(&'static str, SystemTime)> {
	let major_phases = [
		("New Moon", 0.0),
		("First Quarter", 0.25),
		("Full Moon", 0.5),
		("Last Quarter", 0.75),
	];

	let mut phases = Vec::new();
	let mut search_time = current_time;

	for _ in 0..count {
		let mut earliest_phase_time = SystemTime::UNIX_EPOCH;
		let mut next_phase_name = "";
		let mut earliest_duration = std::time::Duration::from_secs(u64::MAX);

		// Find the earliest next phase
		for (name, phase_value) in &major_phases {
			let phase_time = next_phase_time(search_time, *phase_value);
			let duration =
				phase_time.duration_since(search_time).unwrap_or_default();

			if duration < earliest_duration {
				earliest_duration = duration;
				earliest_phase_time = phase_time;
				next_phase_name = name;
			}
		}

		phases.push((next_phase_name, earliest_phase_time));

		// Move search time forward by a small amount to find the next phase
		search_time =
			earliest_phase_time + std::time::Duration::from_secs(3600); // +1 hour
	}

	phases
}

/// Get the descriptive name for a phase value
pub fn phase_name(phase: f64) -> &'static str {
	let phase_index = (phase * 8.0).round() as usize % 8;
	PHASE_NAMES[phase_index]
}

/// Find next waxing crescent moon (phase ~0.125)
pub fn next_waxing_crescent(current_time: SystemTime) -> SystemTime {
	next_phase_time(current_time, 0.125)
}

/// Find next waxing gibbous moon (phase ~0.375)
pub fn next_waxing_gibbous(current_time: SystemTime) -> SystemTime {
	next_phase_time(current_time, 0.375)
}

/// Find next waning gibbous moon (phase ~0.625)
pub fn next_waning_gibbous(current_time: SystemTime) -> SystemTime {
	next_phase_time(current_time, 0.625)
}

/// Find next waning crescent moon (phase ~0.875)
pub fn next_waning_crescent(current_time: SystemTime) -> SystemTime {
	next_phase_time(current_time, 0.875)
}

/// Format SystemTime as a readable string
fn format_time(time: SystemTime) -> String {
	let duration = time.duration_since(SystemTime::UNIX_EPOCH).unwrap();
	let timestamp = duration.as_secs();

	// Days since Unix epoch (January 1, 1970)
	let mut days_since_epoch = timestamp / 86400;
	let seconds_today = timestamp % 86400;
	let hours = seconds_today / 3600;
	let minutes = (seconds_today % 3600) / 60;
	let seconds = seconds_today % 60;

	// Calculate year, month, day (simplified Gregorian calendar)
	let mut year = 1970;
	loop {
		let days_in_year = if is_leap_year(year) { 366 } else { 365 };
		if days_since_epoch >= days_in_year {
			days_since_epoch -= days_in_year;
			year += 1;
		} else {
			break;
		}
	}

	let days_in_months = if is_leap_year(year) {
		[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
	} else {
		[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
	};

	let mut month = 1;
	for &days_in_month in &days_in_months {
		if days_since_epoch >= days_in_month {
			days_since_epoch -= days_in_month;
			month += 1;
		} else {
			break;
		}
	}

	let day = days_since_epoch + 1;

	format!(
		"{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
		year, month, day, hours, minutes, seconds
	)
}

/// Check if a year is a leap year
fn is_leap_year(year: u64) -> bool {
	(year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Calculate time remaining until target time
fn time_until(from: SystemTime, to: SystemTime) -> String {
	match to.duration_since(from) {
		Ok(duration) => {
			let total_seconds = duration.as_secs();
			let days = total_seconds / 86400;
			let hours = (total_seconds % 86400) / 3600;
			let minutes = (total_seconds % 3600) / 60;

			if days > 0 {
				format!("in {}d {}h {}m", days, hours, minutes)
			} else if hours > 0 {
				format!("in {}h {}m", hours, minutes)
			} else {
				format!("in {}m", minutes)
			}
		}
		Err(_) => "in the past".to_string(),
	}
}
