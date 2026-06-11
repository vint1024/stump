use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(LibraryPaths::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(LibraryPaths::Id)
							.integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(LibraryPaths::LibraryId).text().not_null())
					.col(
						ColumnDef::new(LibraryPaths::Path)
							.text()
							.not_null()
							.unique_key(),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-library_paths-library")
							.from(LibraryPaths::Table, LibraryPaths::LibraryId)
							.to(Libraries::Table, Libraries::Id)
							.on_delete(ForeignKeyAction::Cascade)
							.on_update(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("idx-library_paths-library_id")
					.table(LibraryPaths::Table)
					.col(LibraryPaths::LibraryId)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(LibraryPaths::Table).to_owned())
			.await?;

		Ok(())
	}
}

#[derive(DeriveIden)]
enum LibraryPaths {
	Table,
	Id,
	LibraryId,
	Path,
}

#[derive(DeriveIden)]
enum Libraries {
	Table,
	Id,
}
