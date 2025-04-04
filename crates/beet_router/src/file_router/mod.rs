use anyhow::Result;
use beet_rsx::rsx::RsxNode;
use http::Method;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;


pub trait RoutesToRsx {
	async fn routes_to_rsx(&mut self) -> Result<Vec<(RouteInfo, RsxNode)>>;
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteInfo {
	/// the url path
	pub path: RoutePath,
	/// the http method
	#[cfg_attr(
		feature = "serde",
		serde(
			serialize_with = "serialize_method",
			deserialize_with = "deserialize_method"
		)
	)]
	pub method: Method,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoutePath(PathBuf);

impl Default for RoutePath {
	fn default() -> Self { Self(PathBuf::from("/")) }
}

impl std::ops::Deref for RoutePath {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl AsRef<Path> for RoutePath {
	fn as_ref(&self) -> &Path { self.0.as_path() }
}

impl Into<PathBuf> for RoutePath {
	fn into(self) -> PathBuf { self.0 }
}
impl Into<RoutePath> for &str {
	fn into(self) -> RoutePath { RoutePath::new(self) }
}

impl RoutePath {
	pub fn new(path: impl Into<PathBuf>) -> Self { Self(path.into()) }
	/// Creates a route join even if the other route path begins with `/`
	pub fn join(&self, path: &RoutePath) -> Self {
		let new_path =
			path.0.strip_prefix("/").unwrap_or(&path.0).to_path_buf();
		Self(self.0.join(&new_path))
	}
	pub fn inner(&self) -> &Path { &self.0 }
	/// given a local path, return a new [`RoutePath`] with
	/// any extension removed and the path normalized and 'index' removed
	pub fn parse_local_path(local_path: impl AsRef<Path>) -> Result<Self> {
		let mut raw_str = local_path
			.as_ref()
			.to_string_lossy()
			.replace(".rs", "")
			.replace("\\", "/");
		if raw_str.ends_with("index") {
			raw_str = raw_str.replace("index", "");
			// remove trailing `/` from non root paths
			if raw_str.len() > 1 {
				raw_str.pop();
			}
		};
		raw_str = format!("/{}", raw_str);

		Ok(Self(PathBuf::from(raw_str)))
	}
}


#[cfg(feature = "parser")]
impl quote::ToTokens for RouteInfo {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let path = &self.path.to_string_lossy();
		let method = &self.method.to_string();
		tokens.extend(quote::quote! {
					RouteInfo::new(#path, #method)
		});
	}
}


#[derive(Debug, Clone)]
pub struct StaticRouteTree {
	/// the name of this level of the tree, ie the directory.
	/// for the root this is called 'root'
	pub name: String,
	/// all paths available at this level of the tree
	pub paths: Vec<RoutePath>,
	/// All child directories
	pub children: Vec<StaticRouteTree>,
}

impl StaticRouteTree {
	pub fn flatten(&self) -> Vec<RoutePath> {
		let mut paths = self.paths.clone();
		for child in &self.children {
			paths.extend(child.flatten());
		}
		paths
	}
}

// pub struct StaticRouteTreeItem {
// 	pub name: String,
// 	pub path: RoutePath,
// }


#[cfg(feature = "serde")]
fn serialize_method<S>(
	method: &Method,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	serializer.serialize_str(method.as_str())
}

#[cfg(feature = "serde")]
fn deserialize_method<'de, D>(deserializer: D) -> Result<Method, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;
	let s = String::deserialize(deserializer)?;
	Method::from_str(&s).map_err(serde::de::Error::custom)
}


impl RouteInfo {
	/// the method used by `beet_router`
	pub fn new(path: impl Into<PathBuf>, method: &str) -> Self {
		Self {
			path: RoutePath::new(path),
			method: Method::from_str(method).unwrap(),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[cfg(feature = "parser")]
	#[test]
	fn to_tokens() {
		use quote::ToTokens;

		expect(RouteInfo::new("/", "GET").to_token_stream().to_string()).to_be(
			quote::quote! {
				RouteInfo::new("/", "GET")
			}
			.to_string(),
		);
	}
}
