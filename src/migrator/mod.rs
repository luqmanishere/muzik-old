use sea_orm_migration::prelude::*;

mod m20230601_000001_create_basic_table;
mod m20230601_000002_create_junction_tables;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230601_000001_create_basic_table::Migration),
            Box::new(m20230601_000002_create_junction_tables::Migration),
        ]
    }
}
