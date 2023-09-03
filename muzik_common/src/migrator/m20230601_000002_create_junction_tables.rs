use sea_orm_migration::prelude::*;

use super::m20230601_000001_create_basic_table::{Album, Artist, Genre, Song, YoutubePlaylistId};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20230601_000002_create_junction_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SongArtistJunction::Table)
                    .col(
                        ColumnDef::new(SongArtistJunction::Key)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SongArtistJunction::SongId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SongArtistJunction::ArtistId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-song_artist_junction")
                            .from(SongArtistJunction::Table, SongArtistJunction::SongId)
                            .to(Song::Table, Song::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-song_artist_junction")
                            .from(SongArtistJunction::Table, SongArtistJunction::ArtistId)
                            .to(Artist::Table, Artist::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SongAlbumJunction::Table)
                    .col(
                        ColumnDef::new(SongAlbumJunction::Key)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SongAlbumJunction::SongId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SongAlbumJunction::AlbumId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-song_album_junction")
                            .from(SongAlbumJunction::Table, SongAlbumJunction::SongId)
                            .to(Song::Table, Song::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-song_album_junction")
                            .from(SongAlbumJunction::Table, SongAlbumJunction::AlbumId)
                            .to(Album::Table, Album::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SongGenreJunction::Table)
                    .col(
                        ColumnDef::new(SongGenreJunction::Key)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SongGenreJunction::SongId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SongGenreJunction::GenreId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-song_genre_junction")
                            .from(SongGenreJunction::Table, SongGenreJunction::SongId)
                            .to(Song::Table, Song::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-song_genre_junction")
                            .from(SongGenreJunction::Table, SongGenreJunction::GenreId)
                            .to(Genre::Table, Genre::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SongYoutubePlaylistIdJunction::Table)
                    .col(
                        ColumnDef::new(SongYoutubePlaylistIdJunction::Key)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SongYoutubePlaylistIdJunction::SongId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SongYoutubePlaylistIdJunction::YoutubePlaylistIdId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-song_youtube_playlist_id_junction")
                            .from(
                                SongYoutubePlaylistIdJunction::Table,
                                SongYoutubePlaylistIdJunction::SongId,
                            )
                            .to(Song::Table, Song::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-song_youtube_playlist_id_junction")
                            .from(
                                SongYoutubePlaylistIdJunction::Table,
                                SongYoutubePlaylistIdJunction::YoutubePlaylistIdId,
                            )
                            .to(YoutubePlaylistId::Table, YoutubePlaylistId::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SongArtistJunction::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(SongAlbumJunction::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(SongGenreJunction::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(SongYoutubePlaylistIdJunction::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
pub enum SongArtistJunction {
    Table,
    Key,
    SongId,
    ArtistId,
}

#[derive(Iden)]
pub enum SongAlbumJunction {
    Table,
    Key,
    SongId,
    AlbumId,
}

#[derive(Iden)]
pub enum SongGenreJunction {
    Table,
    Key,
    SongId,
    GenreId,
}
#[derive(Iden)]
pub enum SongYoutubePlaylistIdJunction {
    Table,
    Key,
    SongId,
    YoutubePlaylistIdId,
}
