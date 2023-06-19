//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "album")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::song_album_junction::Entity")]
    SongAlbumJunction,
    #[sea_orm(has_many = "super::song::Entity")]
    Song,
}

impl Related<super::song::Entity> for Entity {
    fn to() -> RelationDef {
        super::song_album_junction::Relation::Song.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::song_album_junction::Relation::Album.def().rev())
    }
}

impl Related<super::song_album_junction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SongAlbumJunction.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
