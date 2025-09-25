#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused)]

use beet::prelude::*;
use beet_site::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			BeetPlugins,
			#[cfg(feature = "launch")]
			launch_plugin,
			#[cfg(feature = "server")]
			server_plugin,
			#[cfg(feature = "client")]
			client_plugin,
		))
		.insert_resource(PackageConfig{
			title:"Beet".to_string(),
			..pkg_config!()
		})
		.run();
}

#[cfg(feature = "client")]
fn client_plugin(app: &mut App) {
	app
		.register_type::<ClientIslandRoot<beet_design::templates::BucketList>>()
		.register_type::<ClientIslandRoot<beet_design::mockups::_templates_text_field_mockup::Inner>>()
		.register_type::<ClientIslandRoot<beet_design::mockups::_templates_bucket_list_bucket_id_mockup::Inner>>()
		.register_type::<ClientIslandRoot<beet_design::mockups::_templates_select_mockup::Inner>>()
		.register_type::<ClientIslandRoot<beet_design::mockups::_templates_button_mockup::Inner>>()
		.register_type::<ClientIslandRoot<beet_design::mockups::_templates_form_mockup::Inner>>()
		.register_type::<ClientIslandRoot<ClientCounter>>()
		.register_type::<ClientIslandRoot<ServerCounter>>()
		.register_type::<ClientIslandRoot<ImageGenerator>>()
	/* */;
}
