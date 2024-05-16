use bevy::prelude::*;
use lightyear::prelude::*;

#[derive(Channel)]
pub struct BeetChannel;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BeetMessage(pub String);


pub struct BeetProtocolPlugin;

impl Plugin for BeetProtocolPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<BeetMessage>(ChannelDirection::Bidirectional);

		app.add_channel::<BeetChannel>(ChannelSettings {
			mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
			..default()
		});
	}
}
