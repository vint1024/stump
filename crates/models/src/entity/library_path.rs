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
}
