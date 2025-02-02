use crate::prelude::*;
mod file_router;
mod static_server;
use anyhow::Result;
use beet_rsx::prelude::*;
pub use file_router::*;
use http::Method;
pub use static_server::*;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use sweet::prelude::ReadFile;


