use async_graphql::{InputObject, Result};
use models::{entity::list, shared::enums::EntityVisibility};
use sea_orm::{Set, Unchanged};

/// Input object for creating a list
#[derive(InputObject)]
pub struct CreateListInput {
	/// The name of the list
	pub name: String,
	/// An optional description for the list
	pub description: Option<String>,
	/// The visibility of the list (e.g. public, private)
	pub visibility: EntityVisibility,
}

impl CreateListInput {
	pub fn into_active_model(self, creator_id: String) -> list::ActiveModel {
		list::ActiveModel {
			name: Set(self.name),
			description: Set(self.description),
			visibility: Set(self.visibility),
			creator_id: Set(creator_id),
			..Default::default()
		}
	}
}

// I always pinch myself for not adding patch because full update is so annoying on the frontend,
// so you are welcome future me

/// A patch equivalent of [CreateMetadataProviderConfigInput], i.e. just with optional fields.
#[derive(InputObject)]
pub struct PatchListInput {
	pub name: Option<String>,
	// TODO(list): look into how this will actually work? really what i want is null -> unset, undefined -> no change
	pub description: Option<Option<String>>,
	pub visibility: Option<EntityVisibility>,
}

impl PatchListInput {
	pub async fn apply_to_model(self, model: list::Model) -> Result<list::ActiveModel> {
		Ok(list::ActiveModel {
			id: Unchanged(model.id),
			name: self.name.map(Set).unwrap_or(Unchanged(model.name)),
			description: self
				.description
				.map(Set)
				.unwrap_or(Unchanged(model.description)),
			visibility: self
				.visibility
				.map(Set)
				.unwrap_or(Unchanged(model.visibility)),
			creator_id: Unchanged(model.creator_id),
			created_at: Unchanged(model.created_at),
			..Default::default()
		})
	}
}
