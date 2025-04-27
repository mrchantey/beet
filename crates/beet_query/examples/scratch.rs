use beet_query::as_beet::*;

fn main() {}
struct User {
	// #[iden = "contact"]
	email: String,
}


use beet::exports::sea_query;
use beet::prelude::*;
impl Table for User {
	type Columns = UserCols;
	fn name() -> std::borrow::Cow<'static, str> { "users".into() }
	fn create_table() -> sea_query::TableCreateStatement {
		sea_query::Table::create()
			.table("users")
			.if_not_exists()
			.col(UserCols::Email)
			.to_owned()
	}
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sea_query::Iden)]
enum UserCols {
	Email,
}
impl Columns for UserCols {
	type Table = User;
	fn all() -> Vec<sea_query::ColumnDef> {
		vec![Self::Email.into_column_def()]
	}
}
impl sea_query::IntoColumnDef for UserCols {
	fn into_column_def(self) -> sea_query::ColumnDef {
		match self {
			Self::Email => sea_query::ColumnDef::new_with_type(
				self,
				sea_query::ColumnType::Text,
			)
			.not_null(),
		}
	}
}
