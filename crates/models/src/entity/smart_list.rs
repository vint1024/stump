use async_graphql::{Enum, SimpleObject};
use filter_gen::Ordering;
use sea_orm::{prelude::*, DeriveActiveEnum, EnumIter, QueryOrder};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use super::smart_list_user;
use crate::shared::{
	enums::EntityVisibility,
	ordering::{OrderBy, OrderDirection},
	shared_access::{AccessColumns, EntityColumns, SharedAccessEntity},
};

/// The different filter joiners that can be used in smart lists
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Default,
	EnumIter,
	PartialEq,
	Serialize,
	Deserialize,
	DeriveActiveEnum,
	EnumString,
	Display,
	Enum,
)]
#[sea_orm(
	rs_type = "String",
	rename_all = "SCREAMING_SNAKE_CASE",
	db_type = "String(StringLen::None)"
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum SmartListJoiner {
	#[default]
	And,
	Or,
}

/// The different grouping options for smart lists
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Default,
	EnumIter,
	PartialEq,
	Serialize,
	Deserialize,
	DeriveActiveEnum,
	EnumString,
	Display,
	Enum,
)]
#[sea_orm(
	rs_type = "String",
	rename_all = "SCREAMING_SNAKE_CASE",
	db_type = "String(StringLen::None)"
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum SmartListGrouping {
	#[default]
	ByBooks,
	BySeries,
	ByLibrary,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, SimpleObject, Ordering)]
#[graphql(name = "SmartListModel")]
#[sea_orm(table_name = "smart_lists")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
	pub id: String,
	#[sea_orm(column_type = "Text")]
	pub name: String,
	#[sea_orm(column_type = "Text", nullable)]
	pub description: Option<String>,
	#[sea_orm(column_type = "Blob")]
	#[graphql(skip)]
	pub filters: Vec<u8>,
	#[sea_orm(column_type = "Text")]
	pub joiner: SmartListJoiner,
	#[sea_orm(column_type = "Text")]
	pub default_grouping: SmartListGrouping,
	#[sea_orm(column_type = "Text")]
	pub visibility: EntityVisibility,
	#[sea_orm(column_type = "Text")]
	pub creator_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::smart_list_user::Entity")]
	SmartListUser,
	#[sea_orm(has_many = "super::smart_list_view::Entity")]
	SmartListView,
	#[sea_orm(
		belongs_to = "super::user::Entity",
		from = "Column::CreatorId",
		to = "super::user::Column::Id",
		on_update = "Cascade",
		on_delete = "Cascade"
	)]
	User,
}

impl Related<super::smart_list_user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::SmartListUser.def()
	}
}

impl Related<super::smart_list_view::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::SmartListView.def()
	}
}

impl Related<super::user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::User.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl SharedAccessEntity for Entity {
	type AccessEntity = smart_list_user::Entity;

	fn access_columns() -> AccessColumns<smart_list_user::Entity> {
		AccessColumns {
			entity_id: smart_list_user::Column::SmartListId,
			user_id: smart_list_user::Column::UserId,
			role: smart_list_user::Column::Role,
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
