use crate::{
    error::ModelError,
    traits::{Model, SimpleCrud},
    types::FixedBytesWrapper,
};
use async_trait::async_trait;
use sqlx::{Executor, Postgres};
/// This struct represents an item list in the database.
#[derive(Debug, sqlx::FromRow, Builder)]
pub struct ItemList {
    pub id: FixedBytesWrapper,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// This trait implements the Model trait for the ItemList struct.
impl Model for ItemList {}

#[async_trait]
impl SimpleCrud<FixedBytesWrapper> for ItemList {
    /// This method upserts an item list into the database.
    async fn upsert<'e, E>(&self, schema: &str, executor: E) -> Result<Self, ModelError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let query = format!(
            r#"
            INSERT INTO {}.item_list (id, name, description)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description
            RETURNING 
                id,
                name,
                description
            "#,
            schema,
        );

        sqlx::query_as::<_, ItemList>(&query)
            .bind(self.id.clone())
            .bind(self.name.clone())
            .bind(self.description.clone())
            .fetch_one(executor)
            .await
            .map_err(|e| ModelError::ItemListUpsertError(e.to_string()))
    }

    /// This method finds an item list by its ID in the database.
    async fn find_by_id<'e, E>(
        id: FixedBytesWrapper,
        schema: &str,
        executor: E,
    ) -> Result<Option<Self>, ModelError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let query = format!(
            r#"
            SELECT id, 
                   name, 
                   description 
            FROM {}.item_list 
            WHERE id = $1
            "#,
            schema
        );

        sqlx::query_as::<_, ItemList>(&query)
            .bind(id)
            .fetch_optional(executor)
            .await
            .map_err(|e| ModelError::QueryError(e.to_string()))
    }
}
