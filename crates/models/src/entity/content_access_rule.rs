use sea_orm::{
	entity::prelude::*,
	sea_query::{Query, SimpleExpr},
	Condition,
};
use serde::{Deserialize, Serialize};

use crate::shared::enums::{ContentRuleDimension, ContentRuleMode};

use super::{library_tag, media, media_metadata, media_tag, series, series_metadata, series_tag, tag};

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
/// `media_metadata.genres` ("Fantasy, Horror"). ASCII matching is
/// case-insensitive (SQLite LIKE semantics)
fn comma_list_contains(table: &str, column: &str, value: &str) -> SimpleExpr {
	Expr::cust_with_values(
		format!(
			"(',' || REPLACE(COALESCE(\"{table}\".\"{column}\", ''), ', ', ',') || ',') LIKE ('%,' || ? || ',%')"
		),
		[value.to_string()],
	)
}

fn column_is_unset(table: &str, column: &str) -> SimpleExpr {
	Expr::cust(format!(
		"COALESCE(\"{table}\".\"{column}\", '') = ''"
	))
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
					.add(media::Column::Id.in_subquery(media_ids_with_any_tag()).not())
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
					match_any = match_any
						.add(comma_list_contains("media_metadata", "genres", value))
						.add(comma_list_contains("series_metadata", "genres", value));
				}
				let is_unset = Condition::all()
					.add(column_is_unset("media_metadata", "genres"))
					.add(column_is_unset("series_metadata", "genres"));
				mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset)
			},
			ContentRuleDimension::Publisher => {
				let match_any = Condition::any()
					.add(media_metadata::Column::Publisher.is_in(values.clone()))
					.add(series_metadata::Column::Publisher.is_in(values.clone()));
				let is_unset = Condition::all()
					.add(column_is_unset("media_metadata", "publisher"))
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
					match_any = match_any
						.add(comma_list_contains("series_metadata", "genres", value));
				}
				let is_unset = Condition::all()
					.add(column_is_unset("series_metadata", "genres"));
				mode_condition(rule.mode, rule.restrict_on_unset, match_any, is_unset)
			},
			ContentRuleDimension::Publisher => {
				let match_any = Condition::any()
					.add(series_metadata::Column::Publisher.is_in(values.clone()));
				let is_unset = Condition::all()
					.add(column_is_unset("series_metadata", "publisher"));
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
		let match_any = Condition::any().add(
			super::library::Column::Id.in_subquery(library_ids_with_tags(&values)),
		);
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
