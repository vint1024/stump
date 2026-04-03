use crate::{
	entity::user::AuthUser,
	shared::enums::{EntityVisibility, SharedAccessRole},
};
use async_graphql::ID;
use sea_orm::{entity::prelude::*, Condition, QuerySelect, QueryTrait};

pub struct AccessColumns<AccessEntity: EntityTrait> {
	pub entity_id: <AccessEntity as EntityTrait>::Column,
	pub user_id: <AccessEntity as EntityTrait>::Column,
	pub role: <AccessEntity as EntityTrait>::Column,
}

pub struct EntityColumns<E: EntityTrait> {
	pub id: E::Column,
	pub visibility: E::Column,
	pub creator_id: E::Column,
	pub name: E::Column,
	pub description: E::Column,
}

/// A trait for entities that support shared access control with visibility rules
pub trait SharedAccessEntity: EntityTrait {
	/// The join table entity that links users to this entity.
	/// E.g., for [super::list::Entity], this would be [super::list_user::Entity]
	type AccessEntity: EntityTrait;

	fn access_columns() -> AccessColumns<Self::AccessEntity>;

	fn entity_columns() -> EntityColumns<Self>;
}

fn get_access_condition_base_subquery<E>(user: &AuthUser) -> Select<E::AccessEntity>
where
	E: SharedAccessEntity,
{
	let access_cols = E::access_columns();

	E::AccessEntity::find()
		.select_only()
		.column(access_cols.entity_id)
		.filter(access_cols.user_id.eq(user.id.clone()))
}

fn get_access_condition_base_rule<E>(user: &AuthUser, role: SharedAccessRole) -> Condition
where
	E: SharedAccessEntity,
{
	let access_cols = E::access_columns();
	let entity_cols = E::entity_columns();

	let select = get_access_condition_base_subquery::<E>(user)
		.filter(access_cols.role.gte(role as i32));

	Condition::all().add(entity_cols.id.in_subquery(select.into_query()))
}

fn get_access_condition_for_user_public<E>(
	user: &AuthUser,
	base_rule: Condition,
) -> Condition
where
	E: SharedAccessEntity,
{
	let entity_cols = E::entity_columns();
	let select = get_access_condition_base_subquery::<E>(user);

	Condition::all()
		.add(entity_cols.visibility.eq(EntityVisibility::Public))
		// This asserts the reader rule is present OR there is no rule for the user
		.add(
			Condition::any()
				.add(base_rule)
				.add(entity_cols.id.not_in_subquery(select.into_query())),
		)
}

fn get_access_rule<E>(user: &AuthUser, base_rule: Condition) -> Condition
where
	E: SharedAccessEntity,
{
	let entity_cols = E::entity_columns();

	Condition::any()
		// creator always has access
		.add(entity_cols.creator_id.eq(user.id.clone()))
		// condition where visibility is PUBLIC
		.add(get_access_condition_for_user_public::<E>(
			user,
			base_rule.clone(),
		))
		// condition where visibility is SHARED
		.add(
			Condition::all()
				.add(entity_cols.visibility.eq(EntityVisibility::Shared))
				.add(base_rule.clone()),
		)
		// condition where visibility is PRIVATE
		.add(
			Condition::all()
				.add(entity_cols.visibility.eq(EntityVisibility::Private))
				.add(entity_cols.creator_id.eq(user.id.clone())),
		)
}

pub fn get_access_condition_for_user<E>(
	user: &AuthUser,
	query_all: bool,
	query_mine: bool,
) -> Option<Condition>
where
	E: SharedAccessEntity,
{
	let entity_cols = E::entity_columns();

	if !query_all && !query_mine {
		let base_rule =
			get_access_condition_base_rule::<E>(user, SharedAccessRole::Reader);
		Some(get_access_rule::<E>(user, base_rule))
	} else if query_mine {
		Some(Condition::all().add(entity_cols.creator_id.eq(user.id.clone())))
	} else {
		None
	}
}

pub fn get_search_condition<E>(search: Option<String>) -> Option<Condition>
where
	E: SharedAccessEntity,
{
	search.and_then(|s| {
		if s.is_empty() {
			None
		} else {
			let entity_cols = E::entity_columns();
			Some(
				Condition::any()
					.add(entity_cols.name.contains(&s))
					.add(entity_cols.description.contains(s)),
			)
		}
	})
}

/// Basic DAO methods for entities that implement [SharedAccessEntity]
pub trait SharedAccessEntityDao: SharedAccessEntity {
	fn find_for_user(
		user: &AuthUser,
		query_all: bool,
		query_mine: bool,
		search: Option<String>,
	) -> Select<Self> {
		Self::find().filter(
			Condition::all()
				.add_option(get_search_condition::<Self>(search))
				.add_option(get_access_condition_for_user::<Self>(
					user, query_all, query_mine,
				)),
		)
	}

	fn find_by_id_for_user(user: &AuthUser, id: ID) -> Select<Self> {
		let entity_cols = Self::entity_columns();

		Self::find().filter(
			Condition::all()
				.add_option(get_access_condition_for_user::<Self>(user, false, false))
				.add(entity_cols.id.eq(id.to_string())),
		)
	}
}

// this is a fun rust trick to avoid having to implement the dao for every entity, it will
// automatically implement for structs that impl [SharedAccessEntity]
impl<T: SharedAccessEntity> SharedAccessEntityDao for T {}
