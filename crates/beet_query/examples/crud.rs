//! https://github.com/tursodatabase/limbo/blob/main/bindings/rust/examples/example.rs
use beet_query::as_beet::*;
use sea_query::Expr;

#[derive(Table)]
struct User {
	id: u32,
	email: String,
}


#[tokio::main]
async fn main() {
	// #[cfg(feature = "libsql")]
	let db = Database::new().await.unwrap();
	// #[cfg(not(feature = "libsql"))]
	// let conn = LimboUtils::memory_db().await.unwrap();

	// 1. Initialize Schema
	db.create_table::<User>().await.unwrap();

	// 2. Create Row
	db.insert(UserPartial {
		email: "foo@example.com".into(),
	})
	.await
	.ok()
	.unwrap();


	// 3. Read Row
	let rows = db
		.query(
			User::stmt_select()
				.and_where(Expr::col(UserCols::Email).eq("foo@example.com")),
		)
		.await
		.unwrap();
	assert!(rows.len() == 1);
	assert_eq!(rows[0][1].to_string(), "'foo@example.com'");

	// 4. Update Row
	db.update(User {
		id: 1,
		email: "bar@example.com".into(),
	})
	.await
	.unwrap();

	// 5. Read Changes
	let rows = User::stmt_select()
		.and_where(Expr::col(UserCols::Email).eq("bar@example.com"))
		.query(&db)
		.await
		.unwrap();
	assert!(rows.len() == 1);
	assert_eq!(rows[0][1].to_string(), "'bar@example.com'");
	println!("{}", rows);
}
