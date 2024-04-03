use anyhow::Result;
use beet::action::ActionList;
use beet::reflect::BeetSceneSerde;
use bevy::prelude::*;
use forky_web::download_text;
use forky_web::upload_text;
use forky_web::ResultTJsValueExt;





pub fn download_scene<T: ActionList>(world: &World) -> Result<()> {
	let scene = BeetSceneSerde::<T>::new(world);
	let text = ron::ser::to_string_pretty(&scene, Default::default())?;
	download_text(&text, "scene.ron").anyhow()?;

	Ok(())
}

pub async fn upload_scene<T: ActionList>() -> Result<BeetSceneSerde<T>> {
	let text = upload_text(Some("ron")).await.anyhow()?;
	let scene = ron::de::from_str::<BeetSceneSerde<T>>(&text)?;
	Ok(scene)
}
