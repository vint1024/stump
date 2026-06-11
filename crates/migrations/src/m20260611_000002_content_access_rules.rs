use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(ContentAccessRules::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(ContentAccessRules::Id)
							.integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(ColumnDef::new(ContentAccessRules::UserId).text().not_null())
					.col(
						ColumnDef::new(ContentAccessRules::Dimension)
							.text()
							.not_null(),
					)
					.col(ColumnDef::new(ContentAccessRules::Mode).text().not_null())
					.col(ColumnDef::new(ContentAccessRules::Values).json().not_null())
					.col(
						ColumnDef::new(ContentAccessRules::RestrictOnUnset)
							.boolean()
							.not_null()
							.default(false),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-content_access_rules-user")
							.from(ContentAccessRules::Table, ContentAccessRules::UserId)
							.to(Users::Table, Users::Id)
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
					.name("idx-content_access_rules-user_id")
					.table(ContentAccessRules::Table)
					.col(ContentAccessRules::UserId)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(ContentAccessRules::Table).to_owned())
			.await?;

		Ok(())
	}
}

#[derive(DeriveIden)]
enum ContentAccessRules {
	Table,
	Id,
	UserId,
	Dimension,
	Mode,
	Values,
	RestrictOnUnset,
}

#[derive(DeriveIden)]
enum Users {
	Table,
	Id,
}
