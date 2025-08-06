use aws_sdk_s3::Client;
use beet_core::prelude::*;
use bevy::prelude::*;


#[derive(Resource)]
pub struct S3Client(pub Client);



pub fn s3_fallback(path: RoutePath, _client: Res<S3Client>) -> impl Bundle {

	if path.extension().is_some() {
		todo!("permanent redirect");
	} else {
		// let path = path.join("index.html");
		todo!("download and serve");
	}
	()
}
