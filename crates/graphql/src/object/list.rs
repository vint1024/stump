use async_graphql::{ComplexObject, Context, Result, SimpleObject};
use models::entity::{list, list_media, media, user};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::{
	data::{AuthContext, CoreContext},
	object::user::User,
};

#[derive(Debug, SimpleObject)]
#[graphql(complex)]
pub struct List {
	#[graphql(flatten)]
	pub model: list::Model,
}

impl From<list::Model> for List {
	fn from(entity: list::Model) -> Self {
		Self { model: entity }
	}
}

#[ComplexObject]
impl List {
	// TODO(perf): data loader
	async fn media_count(&self, ctx: &Context<'_>) -> Result<i64> {
		let conn = ctx.data::<CoreContext>()?.conn.as_ref();
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;

		let count = media::Entity::find_for_user(user)
			.inner_join(list_media::Entity)
			.filter(list_media::Column::ListId.eq(self.model.id))
			.count(conn)
			.await?;

		Ok(count.try_into().unwrap_or(0))
	}

	async fn creator(&self, ctx: &Context<'_>) -> Result<User> {
		let conn = ctx.data::<CoreContext>()?.conn.as_ref();
		let creator = user::Entity::find_by_id(self.model.creator_id.clone())
			.one(conn)
			.await?
			.ok_or("Creator not found".to_string())?;
		Ok(creator.into())
	}
}
