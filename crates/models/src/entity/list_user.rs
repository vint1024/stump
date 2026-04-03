use async_graphql::SimpleObject;
use sea_orm::entity::prelude::*;

use crate::shared::enums::SharedAccessRole;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, SimpleObject)]
#[graphql(name = "ListUser")]
#[sea_orm(table_name = "list_users")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i32,
	pub role: SharedAccessRole,
	pub user_id: String,
	pub list_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::list::Entity",
		from = "Column::ListId",
		to = "super::list::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	List,
	#[sea_orm(
		belongs_to = "super::user::Entity",
		from = "Column::UserId",
		to = "super::user::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	SharedUser,
}

impl Related<super::list::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::List.def()
	}
}

impl Related<super::user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::SharedUser.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
