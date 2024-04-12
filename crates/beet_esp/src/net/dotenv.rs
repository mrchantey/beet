use core::convert::TryInto;
use dotenv_codegen::dotenv;

#[derive(Debug, Clone)]
pub struct DotEnv {
	pub wifi_ssid: &'static str,
	pub wifi_pass: &'static str,
}
impl Default for DotEnv {
	fn default() -> Self { Self::new() }
}

impl DotEnv {
	pub fn new() -> Self {
		Self {
			wifi_ssid: dotenv!("WIFI_SSID"),
			wifi_pass: dotenv!("WIFI_PASS"),
		}
	}
	pub fn heapless_ssid(&self) -> heapless::String<32> {
		self.wifi_ssid.try_into().unwrap()
	}
	pub fn heapless_pass(&self) -> heapless::String<64> {
		self.wifi_pass.try_into().unwrap()
	}
}
