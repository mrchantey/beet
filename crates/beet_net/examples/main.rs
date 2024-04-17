use beet_net::prelude::*;
use clap::Parser;
/// We parse the settings.ron file to read the settings, than create the apps and run them
fn main() {
	// cfg_if::cfg_if! {
	// 	if #[cfg(target_family = "wasm")] {
	// 	} else {
	// 	}
	// }
	// let client_id = rand::random::<u64>();
	// let cli = Cli::Client {
	// 		client_id: Some(client_id)
	// };
	let cli = Cli::parse();
	let settings_str = include_str!("../assets/settings.ron");
	let settings = ron::de::from_str::<Settings>(settings_str).unwrap();
	run(settings, cli);
}
