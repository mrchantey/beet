use beet_rsx::as_beet::*;
use bevy::prelude::*;


#[template]
pub fn TemporaryRedirect(href: String) -> impl Bundle {
	OnSpawnBoxed::insert_resource(Response::temporary_redirect(href))
}
#[template]
pub fn PermanentRedirect(href: String) -> impl Bundle {
	OnSpawnBoxed::insert_resource(Response::permanent_redirect(href))
}
