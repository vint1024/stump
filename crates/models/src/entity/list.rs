use async_graphql::SimpleObject;
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{entity::prelude::*, ActiveValue};

use crate::{
	entity::list_user,
	shared::{
		enums::EntityVisibility,
		shared_access::{AccessColumns, EntityColumns, SharedAccessEntity},
	},
};

// TODO(lists): imbue with reading list info instead of maintaining separately, since a reading list is basically the same
// just externally maintained and synced here

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, SimpleObject)]
#[graphql(name = "ListModel")]
#[sea_orm(table_name = "lists")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub name: String,
	pub description: Option<String>,
	pub visibility: EntityVisibility,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: Option<DateTimeWithTimeZone>,
	pub creator_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::list_user::Entity")]
	SharedUser,
	#[sea_orm(
		belongs_to = "super::user::Entity",
		from = "Column::CreatorId",
		to = "super::user::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	Creator,
}

impl Related<super::list_user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::SharedUser.def()
	}
}

impl Related<super::user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Creator.def()
	}
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
	async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
	where
		C: ConnectionTrait,
	{
		if insert {
			if self.id.is_not_set() {
				self.id = ActiveValue::Set(Uuid::new_v4());
			}

			self.created_at = ActiveValue::Set(DateTimeWithTimeZone::from(Utc::now()));
		} else {
			self.updated_at =
				ActiveValue::Set(Some(DateTimeWithTimeZone::from(Utc::now())));
		}

		Ok(self)
	}
}

impl SharedAccessEntity for Entity {
	type AccessEntity = list_user::Entity;

	fn access_columns() -> AccessColumns<list_user::Entity> {
		AccessColumns {
			entity_id: list_user::Column::ListId,
			user_id: list_user::Column::UserId,
			role: list_user::Column::Role,
		}
	}

	fn entity_columns() -> EntityColumns<Self> {
		EntityColumns {
			id: Column::Id,
			visibility: Column::Visibility,
			creator_id: Column::CreatorId,
			name: Column::Name,
			description: Column::Description,
		}
	}
}
