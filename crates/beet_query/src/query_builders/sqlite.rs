use super::QueryBuilder;
use crate::prelude::*;
use anyhow::Result;

pub struct SqliteQueryBuilder;

impl QueryBuilder for SqliteQueryBuilder {
	fn init<T: Table>() -> Result<String> {
		let mut sql = String::new();
		sql.push_str("CREATE TABLE");
		if T::if_not_exists() {
			sql.push_str(" IF NOT EXISTS");
		}
		sql.push_str(" ");
		sql.push_str(&T::name());
		sql.push_str(" (");
		for column in T::Columns::all().into_iter() {
			sql.push_str(&column.name);
			sql.push_str(" ");
			sql.push_str(SqliteQueryBuilder::value_type_to_str(
				column.value_type,
			));
			if !column.optional {
				sql.push_str(" NOT NULL");
			}
			if column.primary_key {
				sql.push_str(" PRIMARY KEY");
			}
			if column.auto_increment {
				sql.push_str(" AUTOINCREMENT");
			}
			if column.unique {
				sql.push_str(" UNIQUE");
			}
			if let Some(default_value) = column.default_value {
				sql.push_str(" DEFAULT ");
				SqliteQueryBuilder::value_to_sql(&mut sql, &default_value);
			}
			sql.push_str(", ");
		}
		// pop the last comma and space
		sql.pop();
		sql.pop();
		sql.push_str(");");
		Ok(sql)
	}
	fn drop<T: Table>() -> Result<String> {
		let mut sql = String::new();
		sql.push_str("DROP TABLE");
		if T::if_not_exists() {
			sql.push_str(" IF EXISTS");
		}
		sql.push_str(" ");
		sql.push_str(&T::name());
		sql.push_str(";");
		Ok(sql)
	}

	fn prepare_insert<T: TableView>() -> Result<String> {
		let mut sql = String::new();
		sql.push_str("INSERT INTO ");
		sql.push_str(&T::Table::name());

		let cols = T::columns()
			.into_iter()
			.map(|col| col.into_column())
			.collect::<Vec<_>>();

		// Add column names
		SqliteQueryBuilder::column_names_to_sql(&mut sql, &cols);

		sql.push_str(" VALUES (");

		// Add parameter placeholders with index
		for (i, _) in cols.iter().enumerate() {
			sql.push_str(&format!("?{}", i + 1));
			sql.push_str(", ");
		}
		// pop the last comma and space
		sql.pop();
		sql.pop();

		sql.push_str(");");
		Ok(sql)
	}

	fn execute_insert<T: TableView>(view: T) -> Result<String> {
		let mut sql = String::new();
		sql.push_str("INSERT INTO ");
		sql.push_str(&T::Table::name());

		// Add column names
		let cols = T::columns()
			.into_iter()
			.map(|col| col.into_column())
			.collect::<Vec<_>>();

		// Add column names
		SqliteQueryBuilder::column_names_to_sql(&mut sql, &cols);

		sql.push_str(" VALUES (");

		// Add values
		for value in view.into_values()?.into_iter() {
			SqliteQueryBuilder::value_to_sql(&mut sql, &value);
			sql.push_str(", ");
		}
		// pop the last comma and space
		sql.pop();
		sql.pop();

		sql.push_str(");");
		Ok(sql)
	}
}
impl SqliteQueryBuilder {
	fn value_type_to_str(value_type: ValueType) -> &'static str {
		match value_type {
			ValueType::Text => "TEXT",
			ValueType::Integer => "INTEGER",
			ValueType::Blob => "BLOB",
			ValueType::Null => "NULL",
			ValueType::Real => "REAL",
		}
	}
	fn value_to_sql(sql: &mut String, value: &Value) {
		match value {
			Value::Integer(i) => sql.push_str(&i.to_string()),
			Value::Text(t) => {
				sql.push('\'');
				sql.push_str(t);
				sql.push('\'');
			}
			Value::Blob(b) => {
				sql.push('\'');
				sql.push_str(&base64::encode(b));
				sql.push('\'');
			}
			Value::Null => sql.push('?'),
			Value::Real(r) => sql.push_str(&r.to_string()),
		}
	}
	/// Converts a list of field names to a sql string,
	/// with the format ` (field1, field2, field3)`,
	/// with leading space, no trailing space or comma
	fn column_names_to_sql(sql: &mut String, column_names: &[Column]) {
		sql.push_str(" (");
		for col in column_names.iter() {
			sql.push_str(&col.name);
			sql.push_str(", ");
		}
		// pop the last comma and space
		sql.pop();
		sql.pop();
		sql.push_str(")");
	}
}



#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Table)]
	#[allow(unused)]
	struct MyTable {
		id: i32,
		name: String,
		age: Option<i32>,
	}

	#[test]
	fn init() {
		expect(&SqliteQueryBuilder::init::<MyTable>().unwrap())
		.to_be("CREATE TABLE IF NOT EXISTS my_table (id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, age INTEGER);");
	}

	#[test]
	fn drop() {
		expect(&SqliteQueryBuilder::drop::<MyTable>().unwrap())
			.to_be("DROP TABLE IF EXISTS my_table;");
	}

	#[test]
	fn prepare_insert() {
		expect(&SqliteQueryBuilder::prepare_insert::<InsertMyTable>().unwrap())
			.to_be("INSERT INTO my_table (id, name, age) VALUES (?1, ?2, ?3);");
	}
	#[test]
	fn execute_insert() {
		let view = InsertMyTable {
			id: None,
			name: "John".to_string(),
			age: Some(30),
		};
		expect(&SqliteQueryBuilder::execute_insert(view).unwrap()).to_be(
			"INSERT INTO my_table (id, name, age) VALUES (?, 'John', 30);",
		);
	}
}
