pub use sea_orm_migration::prelude::*;

mod m20220921_154159_create_prefix_table;
mod m20220928_154159_create_wyr_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220921_154159_create_prefix_table::Migration),
            Box::new(m20220928_154159_create_wyr_table::Migration),
        ]
    }
}
