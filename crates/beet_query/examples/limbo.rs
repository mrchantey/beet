//! https://github.com/tursodatabase/limbo/blob/main/bindings/rust/examples/example.rs
use beet_query::as_beet::*;
use limbo::Builder;

#[derive(Table)]
#[table(name = "users")]
#[allow(unused)]
struct User {
	email: String,
}


#[tokio::main]
async fn main() {
	let db = Builder::new_local(":memory:").build().await.unwrap();
	let conn = db.connect().unwrap();
	User::create_table(&conn).await.unwrap();

	// insert uncached
	User {
		email: "bar@example.com".into(),
	}
	.insert(&conn)
	.await
	.unwrap();
	// User {
	// 	email: "bar@example.com".into(),
	// }
	// .prepare(&conn)
	// .await
	// .unwrap();


	// let mut stmt = conn
	// 	.prepare(&SqliteQueryBuilder::prepare_insert::<InsertUser>().unwrap())
	// 	.await
	// 	.unwrap();

	// stmt.execute(
	// 	InsertUser {
	// 		email: "bar@example.com".into(),
	// 	}
	// 	.into_values()
	// 	.unwrap(),
	// )
	// .await
	// .unwrap();

	let mut stmt = conn
		.prepare("SELECT * FROM users WHERE email = ?1")
		.await
		.unwrap();

	let mut rows = stmt.query(["bar@example.com"]).await.unwrap();

	let row = rows.next().await.unwrap().unwrap();

	let value = row.get_value(0).unwrap();


	println!("Row: {:?}", value);
}
