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

/// A Unicode-case-insensitive `tag.name IN (values)` condition — tag names are
/// user-typed in rules, so "Хоррор" must match a "хоррор" tag
fn tag_name_matches(values: &[String]) -> SimpleExpr {
	let placeholders = values.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
	let params = values.iter().map(|v| v.to_lowercase());
	Expr::cust_with_values(
		format!("ulower(\"tags\".\"name\") IN ({placeholders})"),
		params,
	)
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
		.and_where(tag_name_matches(values))
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
		.and_where(tag_name_matches(values))
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
		.and_where(tag_name_matches(values))
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
/// LIKE metacharacters in it are escaped — and case-insensitively across the
/// full Unicode range: both sides go through the custom `ulower()` function
/// (see [`crate::db`]; SQLite's own `lower()`/LIKE only fold ASCII). COALESCE
/// keeps the expression non-NULL so it is safe to negate for Exclude.
fn comma_list_contains(table: &str, column: &str, value: &str) -> SimpleExpr {
	// Escape so a value like "sci_fi" or "100%" is not treated as a wildcard.
	// Lowercased in Rust — the column side is lowercased by ulower() in SQL
	let escaped = value
		.to_lowercase()
		.replace('\\', "\\\\")
		.replace('%', "\\%")
		.replace('_', "\\_");
	Expr::cust_with_values(
		format!(
			"(',' || ulower(REPLACE(COALESCE(\"{table}\".\"{column}\", ''), ', ', ',')) || ',') LIKE ('%,' || ? || ',%') ESCAPE '\\'"
		),
		[escaped],
	)
}

/// A NULL-safe, Unicode-case-insensitive equality test against a list of
/// values. Returns a guaranteed boolean (never NULL) so it is safe under
/// Exclude's negation — a NULL column must read as "does not match", not
/// "unknown"
fn column_in_values(table: &str, column: &str, values: &[String]) -> SimpleExpr {
	if values.is_empty() {
		return Expr::cust("0 = 1");
	}
	let placeholders = values.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
	let params = values.iter().map(|v| v.to_lowercase());
	Expr::cust_with_values(
		format!(
			"\"{table}\".\"{column}\" IS NOT NULL AND ulower(\"{table}\".\"{column}\") IN ({placeholders})"
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

/// One inheritance level's contribution to a dimension match: how rows at
/// that level match the rule's values, and how they read as "unset"
struct LevelMatch {
	match_any: Condition,
	is_unset: Condition,
}

/// Where the library id lives in the query being filtered: media/series
/// queries don't join `libraries` and reach it through `series.library_id`,
/// while library queries select from the table itself
#[derive(Clone, Copy, PartialEq, Eq)]
enum LibraryIdSource {
	SeriesColumn,
	LibraryTable,
}

impl LibraryIdSource {
	fn expr(self) -> Expr {
		match self {
			Self::SeriesColumn => Expr::col((series::Entity, series::Column::LibraryId)),
			Self::LibraryTable => {
				Expr::col((super::library::Entity, super::library::Column::Id))
			},
		}
	}
}

/// At which inheritance level a dimension is being matched. Tags exist on
/// media, series and libraries; genre/publisher only on media and series
/// metadata
#[derive(Clone, Copy, PartialEq, Eq)]
enum MatchLevel {
	Media,
	Series,
	Library(LibraryIdSource),
}

/// The building block shared by the media/series/library filters: the match
/// and unset conditions one level contributes for one rule's dimension.
/// Returns None when the dimension does not exist at that level
fn level_match(
	level: MatchLevel,
	dimension: ContentRuleDimension,
	values: &[String],
) -> Option<LevelMatch> {
	match (level, dimension) {
		(MatchLevel::Media, ContentRuleDimension::Tag) => Some(LevelMatch {
			match_any: Condition::any()
				.add(media::Column::Id.in_subquery(media_ids_with_tags(values))),
			is_unset: Condition::all().add(
				media::Column::Id
					.in_subquery(media_ids_with_any_tag())
					.not(),
			),
		}),
		(MatchLevel::Series, ContentRuleDimension::Tag) => Some(LevelMatch {
			match_any: Condition::any()
				.add(series::Column::Id.in_subquery(series_ids_with_tags(values))),
			is_unset: Condition::all().add(
				series::Column::Id
					.in_subquery(series_ids_with_any_tag())
					.not(),
			),
		}),
		(MatchLevel::Library(source), ContentRuleDimension::Tag) => Some(LevelMatch {
			match_any: Condition::any()
				.add(source.expr().in_subquery(library_ids_with_tags(values))),
			is_unset: Condition::all()
				.add(source.expr().in_subquery(library_ids_with_any_tag()).not()),
		}),
		(MatchLevel::Media, ContentRuleDimension::Genre) => Some(LevelMatch {
			// Self-contained subqueries — no outer media_metadata join needed
			// (callers like keepReading add their own via find_also_related)
			match_any: Condition::any()
				.add(media::Column::Id.in_subquery(media_ids_with_genre(values))),
			is_unset: Condition::all().add(
				media::Column::Id
					.in_subquery(media_ids_with_any("genres"))
					.not(),
			),
		}),
		(MatchLevel::Series, ContentRuleDimension::Genre) => {
			// Series metadata is matched via the always-joined series_metadata
			let mut match_any = Condition::any();
			for value in values {
				match_any = match_any.add(comma_list_contains(
					"series_metadata",
					"genres",
					value,
				));
			}
			Some(LevelMatch {
				match_any,
				is_unset: Condition::all()
					.add(column_is_unset("series_metadata", "genres")),
			})
		},
		(MatchLevel::Media, ContentRuleDimension::Publisher) => Some(LevelMatch {
			match_any: Condition::any()
				.add(media::Column::Id.in_subquery(media_ids_with_publisher(values))),
			is_unset: Condition::all().add(
				media::Column::Id
					.in_subquery(media_ids_with_any("publisher"))
					.not(),
			),
		}),
		(MatchLevel::Series, ContentRuleDimension::Publisher) => Some(LevelMatch {
			match_any: Condition::any().add(column_in_values(
				"series_metadata",
				"publisher",
				values,
			)),
			is_unset: Condition::all()
				.add(column_is_unset("series_metadata", "publisher")),
		}),
		(MatchLevel::Library(_), _) => None,
	}
}

/// Combine the levels visible to a query into one rule condition: an item
/// matches when ANY level matches, and counts as unset only when EVERY level
/// is unset. Returns None when the dimension exists at none of the levels
fn rule_condition_for_levels(rule: &Model, levels: &[MatchLevel]) -> Option<Condition> {
	let values = rule.values_vec();
	if values.is_empty() {
		return None;
	}
	let mut match_any = Condition::any();
	let mut is_unset = Condition::all();
	let mut any_level = false;
	for level in levels {
		if let Some(level_match) = level_match(*level, rule.dimension, &values) {
			match_any = match_any.add(level_match.match_any);
			is_unset = is_unset.add(level_match.is_unset);
			any_level = true;
		}
	}
	any_level
		.then(|| mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset))
}

/// AND together the conditions of every applicable rule. Returns None when no
/// rule applies at the given levels (no filtering needed)
fn combined_filter(rules: &[Model], levels: &[MatchLevel]) -> Option<Condition> {
	let mut all = Condition::all();
	let mut any_applied = false;
	for rule in rules {
		if let Some(condition) = rule_condition_for_levels(rule, levels) {
			all = all.add(condition);
			any_applied = true;
		}
	}
	any_applied.then_some(all)
}

/// Build the media-level filter for a set of rules. Assumes the query joins
/// `series` (inner) and `series_metadata` (left), as
/// `media::Entity::find_for_user` does — the media side is matched through
/// self-contained subqueries. Tags are matched with inheritance (own + series
/// + library); genre/publisher match the book's own metadata or, when
/// inherited, its series metadata
pub fn media_filter(rules: &[Model]) -> Option<Condition> {
	combined_filter(
		rules,
		&[
			MatchLevel::Media,
			MatchLevel::Series,
			MatchLevel::Library(LibraryIdSource::SeriesColumn),
		],
	)
}

/// Build the series-level filter. Assumes `series_metadata` is joined (left),
/// as `series::Entity::find_for_user` does. Tags match the series' own tags
/// plus its library's tags
pub fn series_filter(rules: &[Model]) -> Option<Condition> {
	combined_filter(
		rules,
		&[
			MatchLevel::Series,
			MatchLevel::Library(LibraryIdSource::SeriesColumn),
		],
	)
}

/// Build the library-level filter. Only tag rules apply to libraries —
/// genre/publisher are book/series concepts and leave libraries visible
pub fn library_filter(rules: &[Model]) -> Option<Condition> {
	combined_filter(rules, &[MatchLevel::Library(LibraryIdSource::LibraryTable)])
}
