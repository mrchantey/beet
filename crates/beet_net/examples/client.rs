use beet_net::prelude::*;


fn main() {
	let cli = Cli::Client {
		client_id: Some(rand::random::<u64>()),
	};

	let settings_str = include_str!("../assets/settings.ron");
	let settings = ron::de::from_str::<Settings>(settings_str).unwrap();
	run(settings, cli);
}
