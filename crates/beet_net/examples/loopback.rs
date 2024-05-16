use beet_net::prelude::*;


fn main() {
	let (mut server, mut client) = loopback_apps();

	std::thread::spawn(move || server.run());
	client.run();
}
