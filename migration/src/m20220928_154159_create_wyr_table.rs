use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Questions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Questions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Questions::Choice1).string().not_null())
                    .col(ColumnDef::new(Questions::Choice2).string().not_null())
                    .col(
                        ColumnDef::new(Questions::Choice1Answers)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Questions::Choice2Answers)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .clone(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Questions::Table).clone())
            .await
    }
}

#[derive(Iden)]
enum Questions {
    Table,
    Id,
    Choice1,
    Choice2,
    Choice1Answers,
    Choice2Answers,
}
