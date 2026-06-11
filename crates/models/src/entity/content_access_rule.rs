use sea_orm::{
	entity::prelude::*,
	sea_query::{Query, SimpleExpr},
	Condition,
};
use serde::{Deserialize, Serialize};

use crate::shared::enums::{ContentRuleDimension, ContentRuleMode};

use super::{library_tag, media, media_metadata, media_tag, series, series_tag, tag};

/// A per-user content access rule. Each rule names a dimension (tag, publisher
/// or genre), a mode and a list of values:
///
/// - `Exclude`: items matching at least one listed value are hidden
/// - `Only`: only items matching at least one listed value are shown; items
///   with no value in that dimension are hidden when `restrict_on_unset` is set
///
/// Multiple rules combine with AND (an item must pass every rule). Tags are
/// inherited: a book is matched against its own tags plus those of its series
/// and library
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "content_access_rules")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	#[sea_orm(column_type = "Text")]
	pub user_id: String,
	#[sea_orm(column_type = "Text")]
	pub dimension: ContentRuleDimension,
	#[sea_orm(column_type = "Text")]
	pub mode: ContentRuleMode,
	#[sea_orm(column_type = "Json")]
	pub values: Json,
	pub restrict_on_unset: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::user::Entity",
		from = "Column::UserId",
		to = "super::user::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	User,
}

impl Related<super::user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::User.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
	/// The rule's values as a plain list of strings
	pub fn values_vec(&self) -> Vec<String> {
		serde_json::from_value(self.values.clone()).unwrap_or_default()
	}
}

impl Entity {
	pub async fn fetch_for_user(
		conn: &DatabaseConnection,
		user_id: &str,
	) -> Result<Vec<Model>, DbErr> {
		Entity::find()
			.filter(Column::UserId.eq(user_id))
			.all(conn)
			.await
	}
}

/// Tag names → subquery of media ids carrying one of those tags
fn media_ids_with_tags(values: &[String]) -> sea_orm::sea_query::SelectStatement {
	Query::select()
		.column(media_tag::Column::MediaId)
		.from(media_tag::Entity)
		.inner_join(
			tag::Entity,
			Expr::col((tag::Entity, tag::Column::Id))
				.equals((media_tag::Entity, media_tag::Column::TagId)),
		)
		.and_where(tag::Column::Name.is_in(values.to_vec()))
		.to_owned()
}

/// Tag names → subquery of series ids carrying one of those tags
fn series_ids_with_tags(values: &[String]) -> sea_orm::sea_query::SelectStatement {
	Query::select()
		.column(series_tag::Column::SeriesId)
		.from(series_tag::Entity)
		.inner_join(
			tag::Entity,
			Expr::col((tag::Entity, tag::Column::Id))
				.equals((series_tag::Entity, series_tag::Column::TagId)),
		)
		.and_where(tag::Column::Name.is_in(values.to_vec()))
		.to_owned()
}

/// Tag names → subquery of library ids carrying one of those tags
fn library_ids_with_tags(values: &[String]) -> sea_orm::sea_query::SelectStatement {
	Query::select()
		.column(library_tag::Column::LibraryId)
		.from(library_tag::Entity)
		.inner_join(
			tag::Entity,
			Expr::col((tag::Entity, tag::Column::Id))
				.equals((library_tag::Entity, library_tag::Column::TagId)),
		)
		.and_where(tag::Column::Name.is_in(values.to_vec()))
		.to_owned()
}

/// Subquery of every media id that has any tag at all
fn media_ids_with_any_tag() -> sea_orm::sea_query::SelectStatement {
	Query::select()
		.column(media_tag::Column::MediaId)
		.from(media_tag::Entity)
		.to_owned()
}

fn series_ids_with_any_tag() -> sea_orm::sea_query::SelectStatement {
	Query::select()
		.column(series_tag::Column::SeriesId)
		.from(series_tag::Entity)
		.to_owned()
}

fn library_ids_with_any_tag() -> sea_orm::sea_query::SelectStatement {
	Query::select()
		.column(library_tag::Column::LibraryId)
		.from(library_tag::Entity)
		.to_owned()
}

/// A comma-separated-list membership test for one value of a TEXT column like
/// `media_metadata.genres` ("Fantasy, Horror"). The value is matched literally —
/// LIKE metacharacters in it are escaped — and case-insensitively for ASCII
/// (SQLite `lower()`/LIKE only fold ASCII; non-ASCII values must match case).
/// COALESCE keeps the expression non-NULL so it is safe to negate for Exclude.
fn comma_list_contains(table: &str, column: &str, value: &str) -> SimpleExpr {
	// Escape so a value like "sci_fi" or "100%" is not treated as a wildcard
	let escaped = value
		.replace('\\', "\\\\")
		.replace('%', "\\%")
		.replace('_', "\\_");
	Expr::cust_with_values(
		format!(
			"(',' || lower(REPLACE(COALESCE(\"{table}\".\"{column}\", ''), ', ', ',')) || ',') LIKE ('%,' || lower(?) || ',%') ESCAPE '\\'"
		),
		[escaped],
	)
}

/// A NULL-safe, ASCII-case-insensitive equality test against a list of values.
/// Returns a guaranteed boolean (never NULL) so it is safe under Exclude's
/// negation — a NULL column must read as "does not match", not "unknown"
fn column_in_values(table: &str, column: &str, values: &[String]) -> SimpleExpr {
	if values.is_empty() {
		return Expr::cust("0 = 1");
	}
	let placeholders = values
		.iter()
		.map(|_| "lower(?)")
		.collect::<Vec<_>>()
		.join(", ");
	let params = values.iter().map(|v| v.to_string());
	Expr::cust_with_values(
		format!(
			"\"{table}\".\"{column}\" IS NOT NULL AND lower(\"{table}\".\"{column}\") IN ({placeholders})"
		),
		params,
	)
}

fn column_is_unset(table: &str, column: &str) -> SimpleExpr {
	Expr::cust(format!("COALESCE(\"{table}\".\"{column}\", '') = ''"))
}

/// Subquery of media ids whose own metadata matches any of the genre values.
/// Self-contained (references media_metadata only inside the subquery) so the
/// outer query needs no media_metadata join — callers like keepReading join it
/// themselves via find_also_related, and a second outer join would be ambiguous
fn media_ids_with_genre(values: &[String]) -> sea_orm::sea_query::SelectStatement {
	let mut cond = Condition::any();
	for value in values {
		cond = cond.add(comma_list_contains("media_metadata", "genres", value));
	}
	Query::select()
		.column(media_metadata::Column::MediaId)
		.from(media_metadata::Entity)
		.cond_where(cond)
		.to_owned()
}

/// Subquery of media ids whose own metadata matches any of the publisher values
fn media_ids_with_publisher(values: &[String]) -> sea_orm::sea_query::SelectStatement {
	Query::select()
		.column(media_metadata::Column::MediaId)
		.from(media_metadata::Entity)
		.and_where(column_in_values("media_metadata", "publisher", values))
		.to_owned()
}

/// Subquery of media ids that have a non-empty value in `column` (genres/publisher)
fn media_ids_with_any(column: &str) -> sea_orm::sea_query::SelectStatement {
	Query::select()
		.column(media_metadata::Column::MediaId)
		.from(media_metadata::Entity)
		.and_where(Expr::cust(format!(
			"COALESCE(\"media_metadata\".\"{column}\", '') <> ''"
		)))
		.to_owned()
}

/// Apply a rule's mode to a "matches at least one value" condition, given a
/// condition describing "has no value at all in this dimension"
fn mode_condition(
	mode: ContentRuleMode,
	restrict_on_unset: bool,
	match_any: Condition,
	is_unset: Condition,
) -> Condition {
	match mode {
		ContentRuleMode::Exclude => Condition::all().add(match_any.not()),
		ContentRuleMode::Only => {
			let mut allowed = Condition::any().add(match_any);
			if !restrict_on_unset {
				allowed = allowed.add(is_unset);
			}
			allowed
		},
	}
}

/// Build the media-level filter for a set of rules. Assumes the query joins
/// `media_metadata` (left), `series` (inner) and `series_metadata` (left), as
/// `media::Entity::find_for_user` does. Tags are matched with inheritance
/// (own + series + library); genre/publisher match the book's own metadata or,
/// when inherited, its series metadata
pub fn media_filter(rules: &[Model]) -> Option<Condition> {
	if rules.is_empty() {
		return None;
	}

	let mut all = Condition::all();
	for rule in rules {
		let values = rule.values_vec();
		if values.is_empty() {
			continue;
		}
		let condition = match rule.dimension {
			ContentRuleDimension::Tag => {
				let match_any = Condition::any()
					.add(media::Column::Id.in_subquery(media_ids_with_tags(&values)))
					.add(series::Column::Id.in_subquery(series_ids_with_tags(&values)))
					.add(
						series::Column::LibraryId
							.in_subquery(library_ids_with_tags(&values)),
					);
				let is_unset = Condition::all()
					.add(
						media::Column::Id
							.in_subquery(media_ids_with_any_tag())
							.not(),
					)
					.add(
						series::Column::Id
							.in_subquery(series_ids_with_any_tag())
							.not(),
					)
					.add(
						series::Column::LibraryId
							.in_subquery(library_ids_with_any_tag())
							.not(),
					);
				mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset)
			},
			ContentRuleDimension::Genre => {
				// Media side via subquery (no media_metadata join needed); series
				// side via the always-joined series_metadata
				let mut match_any = Condition::any()
					.add(media::Column::Id.in_subquery(media_ids_with_genre(&values)));
				for value in &values {
					match_any = match_any.add(comma_list_contains(
						"series_metadata",
						"genres",
						value,
					));
				}
				let is_unset = Condition::all()
					.add(
						media::Column::Id
							.in_subquery(media_ids_with_any("genres"))
							.not(),
					)
					.add(column_is_unset("series_metadata", "genres"));
				mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset)
			},
			ContentRuleDimension::Publisher => {
				let match_any = Condition::any()
					.add(media::Column::Id.in_subquery(media_ids_with_publisher(&values)))
					.add(column_in_values("series_metadata", "publisher", &values));
				let is_unset = Condition::all()
					.add(
						media::Column::Id
							.in_subquery(media_ids_with_any("publisher"))
							.not(),
					)
					.add(column_is_unset("series_metadata", "publisher"));
				mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset)
			},
		};
		all = all.add(condition);
	}

	Some(all)
}

/// Build the series-level filter. Assumes `series_metadata` is joined (left),
/// as `series::Entity::find_for_user` does. Tags match the series' own tags
/// plus its library's tags
pub fn series_filter(rules: &[Model]) -> Option<Condition> {
	if rules.is_empty() {
		return None;
	}

	let mut all = Condition::all();
	for rule in rules {
		let values = rule.values_vec();
		if values.is_empty() {
			continue;
		}
		let condition = match rule.dimension {
			ContentRuleDimension::Tag => {
				let match_any = Condition::any()
					.add(series::Column::Id.in_subquery(series_ids_with_tags(&values)))
					.add(
						series::Column::LibraryId
							.in_subquery(library_ids_with_tags(&values)),
					);
				let is_unset = Condition::all()
					.add(
						series::Column::Id
							.in_subquery(series_ids_with_any_tag())
							.not(),
					)
					.add(
						series::Column::LibraryId
							.in_subquery(library_ids_with_any_tag())
							.not(),
					);
				mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset)
			},
			ContentRuleDimension::Genre => {
				let mut match_any = Condition::any();
				for value in &values {
					match_any = match_any.add(comma_list_contains(
						"series_metadata",
						"genres",
						value,
					));
				}
				let is_unset =
					Condition::all().add(column_is_unset("series_metadata", "genres"));
				mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset)
			},
			ContentRuleDimension::Publisher => {
				let match_any = Condition::any().add(column_in_values(
					"series_metadata",
					"publisher",
					&values,
				));
				let is_unset =
					Condition::all().add(column_is_unset("series_metadata", "publisher"));
				mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset)
			},
		};
		all = all.add(condition);
	}

	Some(all)
}

/// Build the library-level filter. Only tag rules apply to libraries —
/// genre/publisher are book/series concepts and leave libraries visible
pub fn library_filter(rules: &[Model]) -> Option<Condition> {
	if rules.is_empty() {
		return None;
	}

	let mut all = Condition::all();
	let mut any_applied = false;
	for rule in rules {
		let values = rule.values_vec();
		if values.is_empty() || rule.dimension != ContentRuleDimension::Tag {
			continue;
		}
		let match_any = Condition::any()
			.add(super::library::Column::Id.in_subquery(library_ids_with_tags(&values)));
		let is_unset = Condition::all().add(
			super::library::Column::Id
				.in_subquery(library_ids_with_any_tag())
				.not(),
		);
		all = all.add(mode_condition(
			rule.mode,
			rule.restrict_on_unset,
			match_any,
			is_unset,
		));
		any_applied = true;
	}

	any_applied.then_some(all)
}
