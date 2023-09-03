use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20230601_000001_create_basic_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // create Artist table
        manager
            .create_table(
                Table::create()
                    .table(Artist::Table)
                    .col(
                        ColumnDef::new(Artist::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Artist::Name).text().not_null().unique_key())
                    .to_owned(),
            )
            .await?;

        // create Album table
        manager
            .create_table(
                Table::create()
                    .table(Album::Table)
                    .col(
                        ColumnDef::new(Album::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Album::Name).text().not_null().unique_key())
                    .to_owned(),
            )
            .await?;

        // create Genre table
        manager
            .create_table(
                Table::create()
                    .table(Genre::Table)
                    .col(
                        ColumnDef::new(Genre::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Genre::Genre).text().not_null().unique_key())
                    .to_owned(),
            )
            .await?;

        // create youtube playlist id table
        manager
            .create_table(
                Table::create()
                    .table(YoutubePlaylistId::Table)
                    .col(
                        ColumnDef::new(YoutubePlaylistId::Id)
                            .not_null()
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(YoutubePlaylistId::YoutubePlaylistId)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Song::Table)
                    .col(
                        ColumnDef::new(Song::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Song::Title).text().not_null())
                    .col(ColumnDef::new(Song::YoutubeId).text().unique_key())
                    .col(ColumnDef::new(Song::ThumbnailUrl).text())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Artist::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Album::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Genre::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(YoutubePlaylistId::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Song::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
pub enum Artist {
    Table,
    Id,
    Name,
}

#[derive(Iden)]
pub enum Album {
    Table,
    Id,
    Name,
}

#[derive(Iden)]
pub enum Genre {
    Table,
    Id,
    Genre,
}

#[derive(Iden)]
pub enum YoutubePlaylistId {
    Table,
    Id,
    YoutubePlaylistId,
}

// will have only stored id of artist, album,genre,youtubeplaylistid
#[derive(Iden)]
pub enum Song {
    Table,
    Id,
    Title,
    YoutubeId,
    ThumbnailUrl,
    // Added on 26-08-2023
    Path,
}
