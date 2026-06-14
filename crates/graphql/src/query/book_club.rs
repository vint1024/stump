use async_graphql::{Context, Object, Result, ID};
use models::entity::{book_club, book_club_book, book_club_member};
use sea_orm::{prelude::*, ColumnTrait, Condition, QueryFilter, QueryOrder, QuerySelect};

use crate::{
	data::{AuthContext, CoreContext},
	object::{
		book_club::BookClub, book_club_book::BookClubBook,
		book_club_member::BookClubMember,
	},
	pagination::{CursorPaginatedResponse, CursorPagination, CursorPaginationInfo},
};

#[derive(Default)]
pub struct BookClubQuery;

#[Object]
impl BookClubQuery {
	async fn book_clubs(
		&self,
		ctx: &Context<'_>,
		#[graphql(default)] all: Option<bool>,
	) -> Result<Vec<BookClub>> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let conn = ctx.data::<CoreContext>()?.conn.as_ref();

		let models = book_club::Entity::find_all_for_user(all.unwrap_or(false), user)
			.all(conn)
			.await?;

		Ok(models.into_iter().map(BookClub::from).collect())
	}

	async fn book_club_by_id(&self, ctx: &Context<'_>, id: ID) -> Result<BookClub> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let conn = ctx.data::<CoreContext>()?.conn.as_ref();

		let model = book_club::Entity::find_by_id_and_user(id.as_ref(), user)
			.one(conn)
			.await?
			.ok_or_else(|| async_graphql::Error::new("Book club not found"))?;

		Ok(BookClub::from(model))
	}

	async fn book_club_by_slug(
		&self,
		ctx: &Context<'_>,
		slug: String,
	) -> Result<Option<BookClub>> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let conn = ctx.data::<CoreContext>()?.conn.as_ref();

		let model = book_club::Entity::find_by_slug_and_user(slug.as_ref(), user)
			.one(conn)
			.await?;

		Ok(model.map(BookClub::from))
	}

	/// A club's members, cursor-paginated (by id) — for clubs too large to load
	/// the full `members` array. Ordered by id ascending.
	async fn book_club_members(
		&self,
		ctx: &Context<'_>,
		book_club_id: ID,
		#[graphql(default)] pagination: CursorPagination,
	) -> Result<CursorPaginatedResponse<BookClubMember>> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let conn = ctx.data::<CoreContext>()?.conn.as_ref();

		book_club::Entity::find_by_id_and_user(book_club_id.as_ref(), user)
			.one(conn)
			.await?
			.ok_or("Book club not found or you don't have access")?;

		let limit = pagination.limit.min(100);
		let mut query =
			book_club_member::Entity::find_members_accessible_to_user_for_book_club_id(
				user,
				book_club_id.as_ref(),
			)
			.order_by_asc(book_club_member::Column::Id);
		if let Some(after) = &pagination.after {
			query = query.filter(book_club_member::Column::Id.gt(after.as_str()));
		}

		let members = query
			.limit(limit)
			.into_model::<book_club_member::Model>()
			.all(conn)
			.await?;

		let current_cursor = pagination
			.after
			.or_else(|| members.first().map(|m| m.id.clone()));
		let next_cursor = match members.last().map(|m| m.id.clone()) {
			Some(id) if members.len() == limit as usize => Some(id),
			_ => None,
		};

		Ok(CursorPaginatedResponse {
			nodes: members.into_iter().map(BookClubMember::from).collect(),
			cursor_info: CursorPaginationInfo {
				current_cursor,
				next_cursor,
				limit,
			},
		})
	}

	/// A club's archived (completed) books, cursor-paginated by id (a stable
	/// order — ids are random UUIDs, so this is not chronological, but it pages
	/// correctly without dupes/skips) — for clubs with many past reads.
	async fn book_club_previous_books(
		&self,
		ctx: &Context<'_>,
		book_club_id: ID,
		#[graphql(default)] pagination: CursorPagination,
	) -> Result<CursorPaginatedResponse<BookClubBook>> {
		let AuthContext { user, .. } = ctx.data::<AuthContext>()?;
		let conn = ctx.data::<CoreContext>()?.conn.as_ref();

		book_club::Entity::find_by_id_and_user(book_club_id.as_ref(), user)
			.one(conn)
			.await?
			.ok_or("Book club not found or you don't have access")?;

		let limit = pagination.limit.min(100);
		let mut query = book_club_book::Entity::find()
			.filter(book_club_book::Column::BookClubId.eq(book_club_id.as_ref()))
			.filter(book_club_book::Column::CompletedAt.is_not_null())
			// Most-recently-completed first; id breaks ties for a stable keyset.
			.order_by_desc(book_club_book::Column::CompletedAt)
			.order_by_desc(book_club_book::Column::Id);
		// Keyset cursor "<completedAt rfc3339>|<id>": page rows strictly older than
		// the cursor in (completedAt desc, id desc) order — correct across ties.
		// A malformed cursor is an error (not a silent reset to page one).
		if let Some(after) = pagination.after.as_ref() {
			let (ca_str, id_str) =
				after.split_once('|').ok_or("Invalid pagination cursor")?;
			let ca = DateTimeWithTimeZone::parse_from_rfc3339(ca_str)
				.map_err(|_| "Invalid pagination cursor")?;
			query = query.filter(
				Condition::any()
					.add(book_club_book::Column::CompletedAt.lt(ca))
					.add(
						Condition::all()
							.add(book_club_book::Column::CompletedAt.eq(ca))
							.add(book_club_book::Column::Id.lt(id_str)),
					),
			);
		}

		let books = query.limit(limit).all(conn).await?;

		let cursor_of = |b: &book_club_book::Model| {
			format!(
				"{}|{}",
				b.completed_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
				b.id
			)
		};
		let current_cursor = pagination
			.after
			.clone()
			.or_else(|| books.first().map(&cursor_of));
		let next_cursor = match books.last().map(&cursor_of) {
			Some(c) if books.len() == limit as usize => Some(c),
			_ => None,
		};

		Ok(CursorPaginatedResponse {
			nodes: books.into_iter().map(BookClubBook::from).collect(),
			cursor_info: CursorPaginationInfo {
				current_cursor,
				next_cursor,
				limit,
			},
		})
	}
}
