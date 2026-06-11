use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(SeriesMerges::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(SeriesMerges::Id)
							.integer()
							.not_null()
							.auto_increment()
							.primary_key(),
					)
					.col(
						ColumnDef::new(SeriesMerges::TargetSeriesId)
							.text()
							.not_null(),
					)
					.col(
						ColumnDef::new(SeriesMerges::SourcePath)
							.text()
							.not_null()
							.unique_key(),
					)
					.col(ColumnDef::new(SeriesMerges::SourceName).text().not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk-series_merges-target")
							.from(SeriesMerges::Table, SeriesMerges::TargetSeriesId)
							.to(Series::Table, Series::Id)
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
					.name("idx-series_merges-target")
					.table(SeriesMerges::Table)
					.col(SeriesMerges::TargetSeriesId)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(SeriesMerges::Table).to_owned())
			.await?;

		Ok(())
	}
}

#[derive(DeriveIden)]
enum SeriesMerges {
	Table,
	Id,
	TargetSeriesId,
	SourcePath,
	SourceName,
}

#[derive(DeriveIden)]
enum Series {
	Table,
	Id,
}
