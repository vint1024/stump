use sea_orm::entity::prelude::*;

/// An additional root folder for a library. The primary root lives on the
/// library itself (`libraries.path`); rows here extend a library to span
/// multiple folders on disk
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "library_paths")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	#[sea_orm(column_type = "Text")]
	pub library_id: String,
	#[sea_orm(column_type = "Text", unique)]
	pub path: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::library::Entity",
		from = "Column::LibraryId",
		to = "super::library::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	Library,
}

impl Related<super::library::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Library.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
	/// Fetch every additional root path configured for a library
	pub async fn fetch_for_library(
		conn: &DatabaseConnection,
		library_id: &str,
	) -> Result<Vec<String>, DbErr> {
		Ok(Entity::find()
			.filter(Column::LibraryId.eq(library_id))
			.all(conn)
			.await?
			.into_iter()
			.map(|model| model.path)
			.collect())
	}

	/// Fetch the additional root paths for many libraries in one query,
	/// grouped by library id. Libraries without extra roots are absent
	pub async fn fetch_for_libraries(
		conn: &DatabaseConnection,
		library_ids: &[String],
	) -> Result<std::collections::HashMap<String, Vec<String>>, DbErr> {
		let mut by_library: std::collections::HashMap<String, Vec<String>> =
			std::collections::HashMap::new();
		for model in Entity::find()
			.filter(Column::LibraryId.is_in(library_ids.to_vec()))
			.all(conn)
			.await?
		{
			by_library
				.entry(model.library_id)
				.or_default()
				.push(model.path);
		}
		Ok(by_library)
	}
}
