use async_graphql::{InputObject, InputType, OneofObject};
use filter_gen::IntoFilter;
use models::{
	db::UNICODE_LOWER_FN,
	shared::enums::{FileStatus, LibraryType, ReadingStatus},
};
use sea_orm::{
	prelude::DateTimeWithTimeZone,
	sea_query::{Alias, ConditionExpression, Expr, Func, FunctionCall, LikeExpr},
	ColumnTrait, Condition, Value,
};
use serde::{Deserialize, Serialize};

pub mod library;
pub mod log;
pub mod media;
pub mod media_metadata;
pub mod series;
pub mod series_metadata;

// TODO: This probably needs a rewrite to make it more compatible with async-graphql. The big issue is generics
// with input objects. Look at and yoink from seaography for how they are doing things

// Note: See https://github.com/serde-rs/json/issues/501

// NOTE: I originally went for IntoCondition, but that is a trait for sea-query and
// I wanted to avoid conflicts in the naming
pub trait IntoFilter {
	fn into_filter(self) -> sea_orm::Condition;
}

#[derive(OneofObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(concrete(name = "FieldFilterString", params(String)))]
#[graphql(concrete(name = "FieldFilterFileStatus", params(FileStatus)))]
#[serde(rename_all = "camelCase")]
pub enum StringLikeFilter<T>
where
	T: InputType,
{
	Eq(T),
	Neq(T),
	AnyOf(Vec<T>),
	NoneOf(Vec<T>),
	Like(T),
	LikeAnyOf(Vec<T>),
	LikeNoneOf(Vec<T>),
	Contains(T),
	Excludes(T),
	StartsWith(T),
	EndsWith(T),
}

/// Wrap a column in the custom Unicode-lowercasing SQL function so a LIKE
/// against it folds case across the full Unicode range. SQLite's own
/// `lower()`/LIKE only fold ASCII, so without this a search for "рикман" would
/// never match a row stored as "Рикман". `ulower` is registered by
/// `models::db::connect_sqlite`; see [`models::db`].
fn ulower<C>(column: C) -> FunctionCall
where
	C: ColumnTrait,
{
	Func::cust(Alias::new(UNICODE_LOWER_FN)).arg(Expr::col(column.as_column_ref()))
}

/// Lowercase (Unicode-aware) a literal and escape its LIKE metacharacters so it
/// is matched verbatim. Pair with `ESCAPE '\'` on the LIKE so the escapes are
/// honored.
fn lower_escaped<T>(value: T) -> String
where
	T: Into<String>,
{
	value
		.into()
		.to_lowercase()
		.replace('\\', "\\\\")
		.replace('%', "\\%")
		.replace('_', "\\_")
}

pub(crate) fn apply_string_filter<C, T>(
	column: C,
	filter: StringLikeFilter<T>,
) -> Condition
where
	C: ColumnTrait,
	T: InputType + Into<Value> + Into<String>,
{
	match filter {
		StringLikeFilter::Eq(value) => Condition::all().add(column.eq(value)),
		StringLikeFilter::Neq(value) => Condition::all().add(column.ne(value)),
		StringLikeFilter::AnyOf(values) => Condition::all().add(column.is_in(values)),
		StringLikeFilter::NoneOf(values) => {
			Condition::all().add(column.is_not_in(values))
		},
		// The LIKE-family below all fold case across the full Unicode range via
		// ulower(). The SQLite-native column.like()/.contains() only fold ASCII,
		// which left Cyrillic/accented searches (e.g. "рикман", "шитенбург")
		// effectively case-sensitive. Both sides are lowercased: the column by
		// ulower() in SQL, the value by to_lowercase() in Rust.
		StringLikeFilter::Like(value) => Condition::all().add(
			Expr::expr(ulower(column)).like(Into::<String>::into(value).to_lowercase()),
		),
		StringLikeFilter::Contains(value) => Condition::all().add(
			Expr::expr(ulower(column))
				.like(LikeExpr::new(format!("%{}%", lower_escaped(value))).escape('\\')),
		),
		StringLikeFilter::Excludes(value) => Condition::all().add(
			Expr::expr(ulower(column))
				.not_like(Into::<String>::into(value).to_lowercase()),
		),
		StringLikeFilter::StartsWith(value) => Condition::all().add(
			Expr::expr(ulower(column))
				.like(LikeExpr::new(format!("{}%", lower_escaped(value))).escape('\\')),
		),
		StringLikeFilter::EndsWith(value) => Condition::all().add(
			Expr::expr(ulower(column))
				.like(LikeExpr::new(format!("%{}", lower_escaped(value))).escape('\\')),
		),
		StringLikeFilter::LikeAnyOf(values) => {
			values.into_iter().fold(Condition::any(), |acc, value| {
				acc.add(Expr::expr(ulower(column)).like(
					LikeExpr::new(format!("%{}%", lower_escaped(value))).escape('\\'),
				))
			})
		},
		StringLikeFilter::LikeNoneOf(values) => values
			.into_iter()
			.fold(Condition::any(), |acc, value| {
				acc.add(Expr::expr(ulower(column)).like(
					LikeExpr::new(format!("%{}%", lower_escaped(value))).escape('\\'),
				))
			})
			.not(),
	}
}

#[cfg(test)]
mod string_filter_tests {
	use super::*;
	use models::entity::media;
	use sea_orm::{
		sea_query::SqliteQueryBuilder, EntityTrait, QueryFilter, QuerySelect, QueryTrait,
	};

	fn render(filter: StringLikeFilter<String>) -> String {
		media::Entity::find()
			.filter(apply_string_filter(media::Column::Name, filter))
			.select_only()
			.into_query()
			.to_string(SqliteQueryBuilder)
	}

	#[test]
	fn contains_folds_case_via_ulower() {
		let sql = render(StringLikeFilter::Contains("Рикман".to_string()));
		// Column side is ulower()'d and the value is lowercased + LIKE-wrapped.
		assert!(
			sql.contains(r#"ulower("media"."name") LIKE '%рикман%' ESCAPE '\'"#),
			"{sql}"
		);
	}

	#[test]
	fn contains_escapes_like_metacharacters() {
		let sql = render(StringLikeFilter::Contains("100%_x".to_string()));
		assert!(sql.contains(r#"LIKE '%100\%\_x%' ESCAPE '\'"#), "{sql}");
	}

	#[test]
	fn starts_and_ends_with_anchor_pattern() {
		let starts = render(StringLikeFilter::StartsWith("Алан".to_string()));
		assert!(
			starts.contains(r#"ulower("media"."name") LIKE 'алан%' ESCAPE '\'"#),
			"{starts}"
		);
		let ends = render(StringLikeFilter::EndsWith("Лилия".to_string()));
		assert!(
			ends.contains(r#"ulower("media"."name") LIKE '%лилия' ESCAPE '\'"#),
			"{ends}"
		);
	}

	#[test]
	fn eq_is_left_exact() {
		let sql = render(StringLikeFilter::Eq("Exact".to_string()));
		assert!(sql.contains(r#""media"."name" = 'Exact'"#), "{sql}");
		assert!(!sql.contains("ulower"), "{sql}");
	}
}

#[derive(InputObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(concrete(name = "NumericRangeF32", params(f32)))]
#[graphql(concrete(name = "NumericRangeI32", params(i32)))]
#[graphql(concrete(name = "NumericRangeI64", params(i64)))]
#[graphql(concrete(name = "NumericRangeU32", params(u32)))]
#[graphql(concrete(name = "NumericRangeU64", params(u64)))]
#[graphql(concrete(name = "NumericRangeDateTime", params(DateTimeWithTimeZone)))]
#[serde(rename_all = "camelCase")]
pub struct NumericRange<T>
where
	T: InputType,
{
	pub from: T,
	pub to: T,
	pub inclusive: bool,
}

#[derive(OneofObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(concrete(name = "NumericFilterF32", params(f32)))]
#[graphql(concrete(name = "NumericFilterI32", params(i32)))]
#[graphql(concrete(name = "NumericFilterI64", params(i64)))]
#[graphql(concrete(name = "NumericFilterU32", params(u32)))]
#[graphql(concrete(name = "NumericFilterU64", params(u64)))]
#[graphql(concrete(name = "NumericFilterDateTime", params(DateTimeWithTimeZone)))]
#[serde(rename_all = "camelCase")]
pub enum NumericFilter<T>
where
	T: InputType,
	NumericRange<T>: InputType,
{
	Eq(T),
	Neq(T),
	AnyOf(Vec<T>),
	NoneOf(Vec<T>),
	Gt(T),
	Gte(T),
	Lt(T),
	Lte(T),
	Range(NumericRange<T>),
}

pub(crate) fn apply_numeric_filter<C, T>(
	column: C,
	filter: NumericFilter<T>,
) -> impl Into<ConditionExpression>
where
	C: sea_orm::ColumnTrait,
	T: InputType + Into<Value>,
	NumericRange<T>: InputType,
{
	match filter {
		NumericFilter::Eq(value) => column.eq(value),
		NumericFilter::Neq(value) => column.ne(value),
		NumericFilter::AnyOf(values) => column.is_in(values),
		NumericFilter::NoneOf(values) => column.is_not_in(values),
		NumericFilter::Gt(value) => column.gt(value),
		NumericFilter::Gte(value) => column.gte(value),
		NumericFilter::Lt(value) => column.lt(value),
		NumericFilter::Lte(value) => column.lte(value),
		NumericFilter::Range(range) => {
			if range.inclusive {
				column.gte(range.from).and(column.lte(range.to))
			} else {
				column.gt(range.from).and(column.lt(range.to))
			}
		},
	}
}

#[derive(OneofObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(concrete(name = "ComputedFilterReadingStatus", params(ReadingStatus)))]
#[graphql(concrete(name = "ComputedFilterLibraryType", params(LibraryType)))]
#[serde(rename_all = "camelCase")]
pub enum ConceptualFilter<T>
where
	T: InputType,
{
	Is(T),
	IsNot(T),
	IsAnyOf(Vec<T>),
	IsNoneOf(Vec<T>),
}

#[cfg(test)]
mod tests {
	use super::*;
	use models::entity::media;
	use pretty_assertions::assert_eq;
	use sea_orm::{prelude::*, sea_query::SqliteQueryBuilder, QuerySelect, QueryTrait};

	#[test]
	fn test_string_like_any_of() {
		let filter =
			StringLikeFilter::LikeAnyOf(vec!["test".to_string(), "example".to_string()]);
		let condition = apply_string_filter(media::Column::Name, filter);
		let query = media::Entity::find().filter(condition);
		let sql = query
			.select_only()
			.into_query()
			.to_string(SqliteQueryBuilder);

		// ulower()-wrapped + ESCAPE so the match folds case across the full
		// Unicode range (see apply_string_filter).
		assert_eq!(
			sql,
			r#"SELECT  FROM "media" WHERE ulower("media"."name") LIKE '%test%' ESCAPE '\' OR ulower("media"."name") LIKE '%example%' ESCAPE '\'"#
		);
	}

	#[test]
	fn test_string_like_none_of() {
		let filter =
			StringLikeFilter::LikeNoneOf(vec!["test".to_string(), "example".to_string()]);
		let condition = apply_string_filter(media::Column::Name, filter);
		let query = media::Entity::find().filter(condition);
		let sql = query
			.select_only()
			.into_query()
			.to_string(SqliteQueryBuilder);

		assert_eq!(
			sql,
			r#"SELECT  FROM "media" WHERE NOT (ulower("media"."name") LIKE '%test%' ESCAPE '\' OR ulower("media"."name") LIKE '%example%' ESCAPE '\')"#
		);
	}
}
