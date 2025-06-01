mod parse_local_style;
pub use parse_local_style::*;
mod extract_lang_partials;
pub use extract_lang_partials::*;
mod node_portal;
pub use node_portal::*;
mod template_scene;
pub use template_scene::*;
mod templates_to_nodes_rsx;
pub use templates_to_nodes_rsx::*;
mod templates_to_nodes_md;
pub use templates_to_nodes_md::*;
mod templates_to_nodes_rs;
pub use templates_to_nodes_rs::*;
mod template_file;
pub use template_file::*;
mod build_templates_config;
pub mod error;
pub use build_templates_config::*;
mod build_templates_plugin;
pub use build_templates_plugin::*;





pub fn template_types_plugin(app: &mut bevy::prelude::App) {
	app.register_type::<LangPartial>()
		.register_type::<StyleId>()
		.register_type::<NodePortal>()
		.register_type::<NodePortalTarget>();
}
