use sea_orm_migration::prelude::*;

use super::m20230601_000001_create_basic_table::{Album, Artist, Genre, Song, YoutubePlaylistId};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20230826_000003_alter_song_add_path"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Song::Table)
                    .add_column(ColumnDef::new(Song::Path).text())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Song::Table)
                    .drop_column(Song::Path)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
