//! https://github.com/tursodatabase/limbo/blob/main/bindings/rust/examples/example.rs
use beet_query::as_beet::*;
use sea_query::Expr;
use sweet::prelude::PipelineTarget;

#[derive(Table)]
struct User {
	id: u32,
	email: String,
}


#[tokio::main]
async fn main() {
	let conn = LimboUtils::memory_db().await.unwrap();

	// 1. Initialize Schema
	User::create_table(&conn).await.unwrap();

	// 2. Create Row
	UserPartial {
		email: "foo@example.com".into(),
	}
	.insert(&conn)
	.await
	.unwrap();


	// 3. Read Row
	let rows = User::stmt_select()
		.and_where(Expr::col(UserCols::Email).eq("foo@example.com"))
		.xtap(|stmt| {
			println!("{:?}", stmt.build(sea_query::SqliteQueryBuilder))
		})
		.to_owned()
		.query(&conn)
		.await
		.unwrap();
	assert!(rows.len() == 1);

	println!("Rows: {:?}", rows);

	// 4. Update Row
	User {
		id: 1,
		email: "bar@example.com".into(),
	}
	// .update_self(&conn)
	.stmt_update()
	.unwrap()
	.xtap(|stmt| println!("{:?}", stmt.build(sea_query::SqliteQueryBuilder)))
	.execute(&conn)
	// 	.
	.await
	.unwrap();

	let rows = User::stmt_select()
		// .and_where(Expr::col(UserCols::Id).eq(1))
		// .limit(2)
		.and_where(Expr::col(UserCols::Email).eq("bar@example.com"))
		.and_where(Expr::col(UserCols::Email).eq("bar@example.com"))
		.xtap(|stmt| {
			println!("{:?}", stmt.to_string(sea_query::SqliteQueryBuilder))
		})
		.to_owned()
		.query(&conn)
		.await
		.unwrap();
	println!("Rows: {:?}", rows);
	assert!(rows.len() == 1);
}
