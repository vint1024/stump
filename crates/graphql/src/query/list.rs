use crate::{
	data::{AuthContext, CoreContext},
	guard::PermissionGuard,
	object::media::Media,
	pagination::{
		OffsetPaginationInfo, PaginatedResponse, Pagination, PaginationValidator,
	},
};
use async_graphql::{Context, Object, Result, ID};
use models::{
	entity::{list_media, media},
	shared::enums::UserPermission,
};
use sea_orm::{prelude::*, QueryOrder, QuerySelect};

#[derive(Default)]
pub struct ListQuery;

#[Object]
impl ListQuery {
	// the name here is kinda bad, like it sounds like it's listing media and not media within a list
	// it's fine but yknow
	#[graphql(guard = "PermissionGuard::one(UserPermission::ManageLibrary)")]
	async fn list_media(
		&self,
		ctx: &Context<'_>,
		list_id: ID,
		#[graphql(default, validator(custom = "PaginationValidator"))]
		pagination: Pagination,
	) -> Result<PaginatedResponse<Media>> {
		let conn = ctx.data::<CoreContext>()?.conn.as_ref();
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;

		// i have a suspicion that this won't work but we'll see
		let query = media::Entity::find_for_user(user)
			.inner_join(list_media::Entity)
			.filter(list_media::Column::ListId.eq(list_id.to_string()))
			.order_by_asc(list_media::Column::DisplayOrder);

		match pagination.resolve() {
			Pagination::Cursor(info) => {
				// TODO(lists): support this, i see two main options:
				// 1. use the display order as the cursor, which would mean exposing it in the nodes instead of a plain Media
				// 2. use media id, resolve the display order from that
				// either is fine i am just lazy rn
				Err("Cursor pagination not supported for list media".into())
			},
			Pagination::Offset(info) => {
				let count = query.clone().count(conn).await?;

				let models = query
					.offset(info.offset())
					.limit(info.limit())
					.into_model::<media::ModelWithMetadata>()
					.all(conn)
					.await?;

				Ok(PaginatedResponse {
					nodes: models.into_iter().map(Media::from).collect(),
					page_info: OffsetPaginationInfo::new(info, count).into(),
				})
			},
			Pagination::None(_) => {
				let models = query
					.into_model::<media::ModelWithMetadata>()
					.all(conn)
					.await?;
				let count = models.len().try_into()?;

				Ok(PaginatedResponse {
					nodes: models.into_iter().map(Media::from).collect(),
					page_info: OffsetPaginationInfo::unpaged(count).into(),
				})
			},
		}
	}
}
