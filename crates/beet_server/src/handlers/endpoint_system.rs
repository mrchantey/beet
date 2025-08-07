use beet_core::prelude::*;
use bevy::prelude::*;


/// Convenience trait to avoid boilerplate
pub trait EndpointSystem<M> {
	type In: SystemInput;
	type Marker;
	type Out;
	fn into_system(self) -> impl IntoSystem<Self::In, Self::Out, Self::Marker>;
}


impl<T, In, Out, Marker, InErr> EndpointSystem<(Marker, In, InErr, Out)> for T
where
	T: 'static + Send + Sync + Clone + IntoSystem<In, Out, Marker>,
	Out: 'static + Send + Sync + IntoResponse,
	In: 'static + SystemInput,
	for<'a> In::Inner<'a>: TryFrom<Request, Error = InErr>,
	InErr: IntoResponse,
{
	type In = In;
	type Out = Out;
	type Marker = Marker;

	fn into_system(self) -> impl IntoSystem<Self::In, Self::Out, Marker> {
		self
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		fn assert_endpoint_system<I, O, M, M2>(
			_: impl EndpointSystem<M2, In = I, Out = O, Marker = M>,
		) {
		}

		assert_endpoint_system(|| {});
		// assert_endpoint_system(async || {});

		// expect(true).to_be_false();
	}
}
