//! Scene-friendly [`Script`] route marker.
//!
//! [`ExchangeScript`] is a unit marker. It requires a [`ScriptAction`] (which
//! installs the typed `Action<Input, Output>` + `ActionMeta` that runs the
//! sibling [`Script`]) plus the runtime [`ExchangeAction`] used by the router,
//! so the entity becomes a dispatchable route without any post-load hooks.

use crate::prelude::*;
use beet_action::prelude::ScriptAction;
use beet_core::prelude::*;
use beet_net::prelude::FromRequest;
use beet_net::prelude::SerdeFromRequestMarker;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// Reflect-able marker that installs the typed [`ScriptAction`] and the
/// type-erased [`ExchangeAction`] for a [`Script<Input, Output>`] route.
///
/// `M1`/`M2` are [`FromRequest`]/[`ExchangeRouteOut`] markers. The
/// defaults handle the serde blanket case; for custom extractors
/// (eg [`QueryParams`], [`RequestParts`]) instantiate as
/// `ExchangeScript::<Input, Output, _, _>` and let inference pick them.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[reflect(where)]
#[require(
	ScriptAction<Input, Output>,
	ExchangeAction = ExchangeAction::new::<Input, Output, M1, M2>(),
)]
pub struct ExchangeScript<
	Input = (),
	Output = (),
	M1 = SerdeFromRequestMarker,
	M2 = SerdeIntoResponseMarker,
> where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static + Send + Sync + DeserializeOwned + ExchangeRouteOut<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output, M1, M2)>,
}

impl<Input, Output, M1, M2> Default for ExchangeScript<Input, Output, M1, M2>
where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static + Send + Sync + DeserializeOwned + ExchangeRouteOut<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<Input, Output, M1, M2> Clone for ExchangeScript<Input, Output, M1, M2>
where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static + Send + Sync + DeserializeOwned + ExchangeRouteOut<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	fn clone(&self) -> Self { Self::default() }
}

impl<Input, Output, M1, M2> std::fmt::Debug
	for ExchangeScript<Input, Output, M1, M2>
where
	Input: 'static + Send + Sync + Serialize + FromRequest<M1>,
	Output: 'static + Send + Sync + DeserializeOwned + ExchangeRouteOut<M2>,
	M1: 'static + Send + Sync,
	M2: 'static + Send + Sync,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ExchangeScript").finish()
	}
}

#[cfg(test)]
#[cfg(feature = "rhai")]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// An `ExchangeScript` route installs the typed `ScriptAction` (hence an
	/// `ActionMeta`) and the `ExchangeAction`, so the script's output is served
	/// as the route response. Regression: requiring only `Script` left the route
	/// without an `ActionMeta`, so it never joined the `RouteTree`.
	#[beet_core::test]
	async fn exchange_script_route_dispatches() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((default_router(), children![(
				Script::<(), String>::rhai(r#""hello world""#),
				ExchangeScript::<(), String>::default(),
				PathPartial::new("greet"),
			)]))
			.call::<Request, Response>(Request::get("greet"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("hello world");
	}
}
