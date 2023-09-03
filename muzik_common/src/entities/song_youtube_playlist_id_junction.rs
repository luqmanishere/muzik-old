//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "song_youtube_playlist_id_junction")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub key: i32,
    pub song_id: i32,
    pub youtube_playlist_id_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::song::Entity",
        from = "Column::SongId",
        to = "super::song::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Song,
    #[sea_orm(
        belongs_to = "super::youtube_playlist_id::Entity",
        from = "Column::YoutubePlaylistIdId",
        to = "super::youtube_playlist_id::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    YoutubePlaylistId,
}

impl Related<super::song::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Song.def()
    }
}

impl Related<super::youtube_playlist_id::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::YoutubePlaylistId.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}