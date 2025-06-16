mod style_scope;
pub use style_scope::*;
mod lang_content;
pub use lang_content::*;
mod template_directive;
pub use template_directive::*;
mod web_directives;
pub use web_directives::*;
mod rsx_directives;
pub use rsx_directives::*;
pub fn directive_types_plugin(app: &mut bevy::prelude::App) {
    app.register_type::<LangContent>()
        .register_type::<HtmlHoistDirective>()
        .register_type::<ClientLoadDirective>()
        .register_type::<ClientOnlyDirective>()
        .register_type::<SlotChild>()
        .register_type::<SlotTarget>();
}
