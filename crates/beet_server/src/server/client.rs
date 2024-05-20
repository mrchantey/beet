use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws;
use axum_extra::TypedHeader;
use std::net::SocketAddr;

pub struct Client {
	pub socket: ws::WebSocket,
	pub user_agent: String,
	pub connect_info: ConnectInfo<SocketAddr>,
}

impl Client {
	pub fn new(
		socket: ws::WebSocket,
		user_agent: Option<TypedHeader<headers::UserAgent>>,
		connect_info: ConnectInfo<SocketAddr>,
	) -> Self {
		let user_agent = parse_user_agent(user_agent);
		log::info!(
			"New WS Connection\nagent: {user_agent}\naddress: {connect_info:?}"
		);

		Self {
			socket,
			user_agent,
			connect_info,
		}
	}
}


fn parse_user_agent(
	user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> String {
	if let Some(TypedHeader(user_agent)) = user_agent {
		user_agent.to_string()
	} else {
		String::from("Unknown User Agent")
	}
}
