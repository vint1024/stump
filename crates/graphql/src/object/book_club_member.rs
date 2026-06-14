use async_graphql::{
	dataloader::DataLoader, ComplexObject, Context, Result, SimpleObject,
};
use models::{entity::book_club_member, shared::book_club::BookClubMemberRole};

use crate::{data::ServiceContext, loader::user::UserLoader, object::user::User};

#[derive(Debug, SimpleObject)]
#[graphql(complex)]
pub struct BookClubMember {
	#[graphql(flatten)]
	model: book_club_member::Model,
}

impl From<book_club_member::Model> for BookClubMember {
	fn from(model: book_club_member::Model) -> Self {
		Self { model }
	}
}

#[ComplexObject]
impl BookClubMember {
	async fn avatar_url(&self, ctx: &Context<'_>) -> Result<Option<String>> {
		let service = ctx.data::<ServiceContext>()?;
		let loader = ctx.data::<DataLoader<UserLoader>>()?;
		let user = loader
			.load_one(self.model.user_id.clone())
			.await?
			.ok_or_else(|| async_graphql::Error::new("User not found"))?;

		if user.avatar_path.is_none() {
			return Ok(None);
		}

		Ok(Some(service.format_url(format!(
			"/api/v2/users/{}/avatar",
			self.model.user_id
		))))
	}

	async fn username(&self, ctx: &Context<'_>) -> Result<String> {
		if let Some(ref username) = self.model.display_name {
			return Ok(username.clone());
		}

		let loader = ctx.data::<DataLoader<UserLoader>>()?;
		let user = loader
			.load_one(self.model.user_id.clone())
			.await?
			.ok_or_else(|| async_graphql::Error::new("User not found"))?;

		Ok(user.username)
	}

	async fn user(&self, ctx: &Context<'_>) -> Result<User> {
		let loader = ctx.data::<DataLoader<UserLoader>>()?;
		let model = loader
			.load_one(self.model.user_id.clone())
			.await?
			.ok_or_else(|| async_graphql::Error::new("User not found"))?;

		Ok(User::from(model))
	}

	async fn is_creator(&self) -> bool {
		self.model.role == BookClubMemberRole::Creator
	}
}
