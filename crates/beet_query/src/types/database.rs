use crate::prelude::*;
use anyhow::Result;
use sea_query::Expr;
use sweet::prelude::*;


#[derive(Clone)]
pub struct Database {
	pub connection: Connection,
	pub statement_cache: CachedStatementMap,
}



impl Database {
	pub async fn new() -> Result<Self> {
		let connection = Connection::new().await?;
		let statement_cache = Default::default();
		Ok(Self {
			connection,
			statement_cache,
		})
	}

	async fn get_or_prepare(&self, sql: &str) -> Result<CachedStatement> {
		self.statement_cache
			.get_or_prepare(sql, || Box::pin(self.connection.prepare(sql)))
			.await
	}

	/// Execute a statement:
	///
	/// ## Caching
	/// The caching strategy is automatically determined based on
	/// the statement type:
	/// - [`Schema`](sea_query::SchemaStatementBuilder) statements like
	///  creating tables or indexes are executed without caching
	/// - [`Query`](sea_query::QueryStatementBuilder) statements like
	/// `SELECT`, `INSERT`, `UPDATE` and `DELETE are cached
	pub async fn execute(&self, stmt: &impl Statement) -> Result<()> {
		if stmt.statement_type() == StatementType::Schema {
			self.connection.execute_uncached(stmt).await
		} else {
			self.execute_cached(stmt).await
		}
	}
	pub async fn query(&self, stmt: &impl Statement) -> Result<Rows> {
		let (sql, values) =
			stmt.build(&*self.connection.statement_builder())?;
		self.get_or_prepare(&sql).await?.query(values).await
	}

	/// Execute a statement and automatically cache it for next call. This
	/// is usually used by [`Query`](sea_query::QueryStatementBuilder) statements
	/// like `SELECT`, `INSERT`, `UPDATE` and `DELETE`.
	async fn execute_cached(&self, stmt: &impl Statement) -> Result<()> {
		let (sql, values) =
			stmt.build(&*self.connection.statement_builder())?;
		self.get_or_prepare(&sql).await?.execute(values).await
	}



	/// Execute a `CREATE TABLE` statement with this table's [`stmt_create_table`](Table::stmt_create_table)
	pub async fn create_table<T: Table>(&self) -> Result<()> {
		self.execute(&T::stmt_create_table()).await
	}

	pub async fn insert<T: TableView>(&self, value: T) -> Result<()> {
		self.execute(&value.stmt_insert()?).await
	}

	/// Find a row by its primary key
	pub async fn find<T: TableView>(&self, key_val: T::PrimaryKey) -> Result<T>
	where
		T::PrimaryKey: ConvertValue,
	{
		T::stmt_select()
			.and_where(T::expr_primary_key_eq(key_val)?)
			.query(&self)
			.await?
			.exactly_one()?
			.xmap(T::from_row)?
			.xok()
	}


	pub async fn update<T: TableView>(&self, value: T) -> Result<()> {
		let kvp = value.primary_kvp()?;
		value
			.stmt_update()?
			.and_where(
				Expr::col(kvp.0)
					.eq(kvp.1.into_other::<sea_query::SimpleExpr>()?),
			)
			.execute(self)
			.await
	}
	pub async fn delete<T: TableView>(
		&self,
		key_val: T::PrimaryKey,
	) -> Result<()>
	where
		T::PrimaryKey: ConvertValue,
	{
		T::stmt_delete()
			.and_where(T::expr_primary_key_eq(key_val)?)
			.execute(self)
			.await
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Debug, Clone, PartialEq, Table)]
	struct User {
		id: i32,
		name: String,
		email: String,
	}


	#[sweet::test]
	async fn works() {
		let db = Database::new().await.unwrap();

		db.create_table::<User>().await.unwrap();

		let user = User {
			id: 1,
			name: "IronMan".to_string(),
			email: "tony@starkenterprize.com".to_string(),
		};

		// 1. CREATE
		db.insert(user.clone()).await.unwrap();

		// 2. READ
		db.find::<User>(1).await.unwrap().xpect().to_be(user);

		// 3. UPDATE
		db.update(User {
			id: 1,
			name: "WonderWoman".to_string(),
			email: "hiddencity@amazonrainforest.com".to_string(),
		})
		.await
		.unwrap();

		db.find::<User>(1)
			.await
			.unwrap()
			.xmap(|u| u.name)
			.xpect()
			.to_be("WonderWoman".to_string());

		// 4. DELETE
		db.delete::<User>(1).await.unwrap();
		db.find::<User>(1).await.xpect().to_be_err();
	}
}
