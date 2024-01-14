use std::fmt;

use sqlx::{Encode, MySql, QueryBuilder, Type};

use crate::params::{Limit, Offset};

/// A helper for building conditional queries.
///
/// # Example
/// ```ignore
/// let mut query = QueryBuilder::new("SELECT * FROM table");
/// let mut filter = Filter::new();
///
/// if let Some(param) = some_param {
///     query.push(filter).push(" some_param = ").push_bind(param);
///     filter.switch();
/// }
///
/// if let Some(param) = other_param {
///     query.push(filter).push(" other_param = ").push_bind(param);
///     filter.switch();
/// }
///
/// // ...
///
/// let results = query.build().execute(&mut conn).await?;
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub enum Filter {
	/// SQL `WHERE` clause.
	#[default]
	Where,

	/// SQL `AND` clause.
	And,
}

impl Filter {
	/// Creates a new [`WHERE`] [`Filter`].
	///
	/// [`WHERE`]: type@Filter::Where
	pub const fn new() -> Self {
		Self::Where
	}

	/// Switches to [`AND`].
	///
	/// [`AND`]: type@Filter::And
	pub fn switch(&mut self) {
		*self = Self::And;
	}
}

impl fmt::Display for Filter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Filter::Where => " WHERE ",
			Filter::And => " AND ",
		})
	}
}

/// Pushes `LIMIT` and `OFFSET` parameters into the given `query`.
pub fn push_limit(limit: Limit, offset: Offset, query: &mut QueryBuilder<'_, MySql>) {
	query
		.push(" LIMIT ")
		.push_bind(limit)
		.push(" OFFSET ")
		.push_bind(offset);
}

/// Pushes a tuple of `items` into a `SELECT * FROM table WHERE field IN (...)` query.
///
/// # Example
///
/// ```ignore
/// let mut query = QueryBuilder::new("SELECT * FROM table WHERE field IN");
/// let values = [1, 2, 3];
///
/// push_tuple(&values, &mut query);
///
/// let sql = query.sql();
///
/// // The values of `1`, `2`, and `3` will be bound to this query when executed.
/// assert_eq!(sql, "SELECT * FROM table WHERE field IN (?, ?, ?)");
/// ```
pub fn push_tuple<'q, 'args, I>(items: I, query: &'q mut QueryBuilder<'args, MySql>)
where
	I: IntoIterator,
	I::Item: Encode<'args, MySql> + Type<MySql> + Send + 'args,
{
	query.push(" (");

	let mut separated = query.separated(", ");

	for item in items {
		separated.push_bind(item);
	}

	separated.push_unseparated(") ");
}