use async_graphql::SimpleObject;
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{entity::prelude::*, ActiveValue};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, SimpleObject)]
#[graphql(name = "ListUser")]
#[sea_orm(table_name = "list_media")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub media_id: String,
	#[sea_orm(primary_key, auto_increment = false)]
	pub list_id: String,
	pub display_order: i32,
	pub added_at: DateTimeWithTimeZone,
	pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::media::Entity",
		from = "Column::MediaId",
		to = "super::media::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	Media,
	#[sea_orm(
		belongs_to = "super::list::Entity",
		from = "Column::ListId",
		to = "super::list::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	List,
}

impl Related<super::list::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::List.def()
	}
}

impl Related<super::media::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Media.def()
	}
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
	async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
	where
		C: ConnectionTrait,
	{
		if insert {
			self.added_at = ActiveValue::Set(DateTimeWithTimeZone::from(Utc::now()));
		} else {
			self.updated_at =
				ActiveValue::Set(Some(DateTimeWithTimeZone::from(Utc::now())));
		}

		Ok(self)
	}
}
