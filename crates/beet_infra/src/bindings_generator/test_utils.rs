#![allow(
	unused_imports,
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals
)]
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap as Map;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct config {
	pub data: Option<Vec<datasource_root>>,
	pub provider: Option<Vec<provider_root>>,
	pub resource: Option<Vec<resource_root>>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum datasource_root {
	test_data_source_a(Vec<Map<String, Vec<test_data_source_a_details>>>),
	test_data_source_b(Vec<Map<String, Vec<test_data_source_b_details>>>),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum provider_root {
	test_provider(Vec<test_provider_details>),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum resource_root {
	test_resource_a(Vec<Map<String, Vec<test_resource_a_details>>>),
	test_resource_b(Vec<Map<String, Vec<test_resource_b_details>>>),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct test_data_source_a_details {
	pub description: Option<String>,
	pub id: Option<String>,
	pub name: String,
	pub users: Option<Vec<String>>,
	pub datasource_a_type: Option<
		Vec<test_data_source_a_data_source_block_type_datasource_a_type>,
	>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct test_data_source_b_details {
	pub id: Option<String>,
	pub name: String,
	pub r#type: String,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct test_provider_details {
	pub api_token: String,
	pub backoff: Option<bool>,
	pub base_url: Option<String>,
	pub max_retries: Option<i64>,
	pub max_wait_seconds: Option<i64>,
	pub min_wait_seconds: Option<i64>,
	pub org_name: String,
	pub parallelism: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct test_resource_a_details {
	pub client_whitelist: Vec<String>,
	pub description: String,
	pub id: Option<String>,
	pub name: String,
	pub priority: i64,
	pub status: Option<String>,
	pub r#type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct test_resource_b_details {
	pub description: Option<String>,
	pub groups_included: Option<Vec<String>>,
	pub id: Option<String>,
	pub name: String,
	pub priority: Option<i64>,
	pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "datasource_a_type")]
pub struct test_data_source_a_data_source_block_type_datasource_a_type {
	pub filter_type: Option<String>,
	pub filter_value: Option<String>,
	pub name: String,
	pub namespace: Option<String>,
	pub r#type: Option<String>,
	pub values: Option<Vec<String>>,
}
