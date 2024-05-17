use anyhow::bail;
use anyhow::Result;
use core::time::Duration;
use esp_idf_hal::rmt::*;

pub struct Rgb {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

// const WS2812_T0H_NS: Duration = Duration::from_nanos(400);
// const WS2812_T0L_NS: Duration = Duration::from_nanos(850);
// const WS2812_T1H_NS: Duration = Duration::from_nanos(800);
// const WS2812_T1L_NS: Duration = Duration::from_nanos(450);
const WS2812_T0H_NS: Duration = Duration::from_nanos(350);
const WS2812_T0L_NS: Duration = Duration::from_nanos(800);
const WS2812_T1H_NS: Duration = Duration::from_nanos(700);
const WS2812_T1L_NS: Duration = Duration::from_nanos(600);


impl Rgb {
	pub fn new(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
	/// Converts hue, saturation, value to RGB
	pub fn from_hsv(h: u32, s: u32, v: u32) -> Result<Self> {
		if h > 360 || s > 100 || v > 100 {
			bail!("The given HSV values are not in valid range");
		}
		let s = s as f64 / 100.0;
		let v = v as f64 / 100.0;
		let c = s * v;
		let x = c * (1.0 - (((h as f64 / 60.0) % 2.0) - 1.0).abs());
		let m = v - c;
		let (r, g, b) = match h {
			0..=59 => (c, x, 0.0),
			60..=119 => (x, c, 0.0),
			120..=179 => (0.0, c, x),
			180..=239 => (0.0, x, c),
			240..=299 => (x, 0.0, c),
			_ => (c, 0.0, x),
		};
		Ok(Self {
			r: ((r + m) * 255.0) as u8,
			g: ((g + m) * 255.0) as u8,
			b: ((b + m) * 255.0) as u8,
		})
	}

	/// BLOCKING transmition
	pub fn transmit(&self, tx: &mut TxRmtDriver) -> Result<()> {
		let color = self.into_tx_value();
		let ticks_hz = tx.counter_clock()?;
		let (t0h, t0l, t1h, t1l) = (
			Pulse::new_with_duration(ticks_hz, PinState::High, &WS2812_T0H_NS)?,
			Pulse::new_with_duration(ticks_hz, PinState::Low, &WS2812_T0L_NS)?,
			Pulse::new_with_duration(ticks_hz, PinState::High, &WS2812_T1H_NS)?,
			Pulse::new_with_duration(ticks_hz, PinState::Low, &WS2812_T1L_NS)?,
		);
		let mut signal = FixedLengthSignal::<24>::new();
		for i in (0..24).rev() {
			let p = 2_u32.pow(i);
			let bit: bool = p & color != 0;
			let (high_pulse, low_pulse) =
				if bit { (t1h, t1l) } else { (t0h, t0l) };
			signal.set(23 - i as usize, &(high_pulse, low_pulse))?;
		}
		tx.start_blocking(&signal)?;
		Ok(())
	}

	/// Convert RGB to u32 color value
	///
	/// e.g. rgb: (1,2,4)
	/// G        R        B
	/// 7      0 7      0 7      0
	/// 00000010 00000001 00000100
	pub fn into_tx_value(&self) -> u32 {
		((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
	}
}
