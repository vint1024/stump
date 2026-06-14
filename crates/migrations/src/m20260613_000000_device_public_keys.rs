use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(DevicePublicKeys::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(DevicePublicKeys::Id)
							.text()
							.not_null()
							.primary_key(),
					)
					.col(ColumnDef::new(DevicePublicKeys::UserId).text().not_null())
					.col(ColumnDef::new(DevicePublicKeys::DeviceId).text().not_null())
					.col(
						ColumnDef::new(DevicePublicKeys::PublicKey)
							.binary()
							.not_null(),
					)
					.col(
						ColumnDef::new(DevicePublicKeys::CreatedAt)
							.timestamp()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-device-public-keys-user")
							.from(DevicePublicKeys::Table, DevicePublicKeys::UserId)
							.to(Users::Table, Users::Id)
							.on_delete(ForeignKeyAction::Cascade)
							.on_update(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		// One key per (user, device): re-registration updates the existing row.
		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("idx-device-public-keys-user-device")
					.table(DevicePublicKeys::Table)
					.col(DevicePublicKeys::UserId)
					.col(DevicePublicKeys::DeviceId)
					.unique()
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(DevicePublicKeys::Table).to_owned())
			.await
	}
}

#[derive(Iden)]
enum DevicePublicKeys {
	Table,
	Id,
	UserId,
	DeviceId,
	PublicKey,
	CreatedAt,
}

#[derive(Iden)]
enum Users {
	Table,
	Id,
}
