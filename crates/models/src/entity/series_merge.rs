use std::collections::HashMap;

use sea_orm::entity::prelude::*;

/// A record of a series merge: the folder at `source_path` used to be its own
/// series (named `source_name`), and its books now belong to the target series.
/// The scanner consults this map so the source folder is not re-created as a
/// series on subsequent scans, and un-merging restores the original series
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "series_merges")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	#[sea_orm(column_type = "Text")]
	pub target_series_id: String,
	#[sea_orm(column_type = "Text", unique)]
	pub source_path: String,
	#[sea_orm(column_type = "Text")]
	pub source_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::series::Entity",
		from = "Column::TargetSeriesId",
		to = "super::series::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	TargetSeries,
}

impl Related<super::series::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::TargetSeries.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
	/// Build the merge map (source_path → target_series_id) for every series
	/// in the given library
	pub async fn fetch_map_for_library(
		conn: &DatabaseConnection,
		library_id: &str,
	) -> Result<HashMap<String, String>, DbErr> {
		let rows = Entity::find()
			.inner_join(super::series::Entity)
			.filter(super::series::Column::LibraryId.eq(library_id))
			.all(conn)
			.await?;
		Ok(rows
			.into_iter()
			.map(|row| (row.source_path, row.target_series_id))
			.collect())
	}
}
