#![allow(unused)]
use beet_query::as_beet::*;
use sea_query::SqliteQueryBuilder;

fn main() {
	let create = User::stmt_create_table().to_string(SqliteQueryBuilder);

	assert_eq!(
		create,
		"CREATE TABLE IF NOT EXISTS \"users\" ( \"contact\" text DEFAULT 'foobar' NOT NULL )"
	);
	println!("{}", create);
}

#[derive(Table)]
#[table(name = "users")]
// #[allow(unused)]
struct User {
	id: i32,
	#[iden = "contact"]
	#[field(default = "foobar")]
	email: String,
}

#[derive(Default, TableView)]
#[table_view(table = User)]
struct UserEmail {
	email: String,
}
