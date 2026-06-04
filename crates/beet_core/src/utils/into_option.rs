/// A trait for converting a type into an `Option<T>`,
/// including the unit type, with a marker to specify the conversion strategy.
pub trait IntoOption<T, M> {
	/// Converts this value into an `Option<T>` according to the specified marker.
	fn into_option(self) -> Option<T>;
}
/// Marker type for converting a type that implements `Into<T>` into an `Option<T>`.
pub struct TypeIntoOptionMarker;

impl<T, U> IntoOption<T, TypeIntoOptionMarker> for U
where
	U: Into<T>,
{
	fn into_option(self) -> Option<T> { Some(self.into()) }
}
/// Marker type for converting an `Option<U>` where `U: Into<T>` into an `Option<T>`.
pub struct OptionIntoOptionMarker;

impl<T, U> IntoOption<T, OptionIntoOptionMarker> for Option<U>
where
	U: Into<T>,
{
	fn into_option(self) -> Option<T> { self.map(|val| val.into()) }
}
/// Marker type for converting the unit type `()` into `Option<T>`, always returning `None`.
pub struct UnitIntoOptionMarker;

impl<T> IntoOption<T, UnitIntoOptionMarker> for () {
	fn into_option(self) -> Option<T> { None }
}
