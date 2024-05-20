use std::time::Duration;

pub type UserId = u32;
pub type ClientId = u32;
pub type LobbyId = u32;
pub type ChannelId = u32;


pub const SOCKET_INTERVAL: Duration = Duration::from_millis(100);
