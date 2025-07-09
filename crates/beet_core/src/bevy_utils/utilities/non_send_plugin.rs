use bevy::app::App;



/// Consistent interface for `!Send` types that want to be a [`Plugin`]
pub trait NonSendPlugin: Sized {
	fn build(self, app: &mut App);
}
