use crate::bevybail;
use bevy::app::AppExit;

#[extend::ext(name=AppExitExt)]
pub impl AppExit {
	fn into_result(self) -> bevy::prelude::Result {
		match self {
			AppExit::Success => Ok(()),
			AppExit::Error(err) => {
				bevybail!("Application exited with error: {}", err)
			}
		}
	}
}
