use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;


#[template]
pub fn TemporaryRedirect(href: String) -> impl Bundle {
	OnSpawn::insert_resource(Response::temporary_redirect(href))
}
#[template]
pub fn PermanentRedirect(href: String) -> impl Bundle {
	OnSpawn::insert_resource(Response::permanent_redirect(href))
}
