use crate::error::{Error, Result};

pub async fn new_object_id(
    tx: &'_ mut sqlx::Transaction<'_, sqlx::Postgres>,
    object_type: &str,
) -> Result<i64, sqlx::Error> {
    let id = sqlx::query_scalar!(
        "INSERT INTO object_ids (object_id, type) VALUES (DEFAULT, $1::object_type) RETURNING object_id",
        object_type as _
    )
    .fetch_one(&mut *tx)
    .await?;
    Ok(id)
}

pub async fn new_object_id_with_value(
    tx: &'_ mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Option<&i64>,
    object_type: &str,
    allow_existing: bool,
) -> Result<i64> {
    let result = match (id, allow_existing) {
        (Some(id), false) => {
            sqlx::query_scalar!(
                "INSERT INTO object_ids (object_id, type) VALUES ($1, $2::object_type)",
                id,
                object_type as _
            )
            .execute(&mut *tx)
            .await?;
            *id
        }
        (Some(id), true) => {
            let result = sqlx::query_scalar!(
                "INSERT INTO object_ids (object_id, type) VALUES ($1, $2::object_type) ON CONFLICT DO NOTHING",
                id,
                object_type as _
            )
            .fetch_optional(&mut *tx)
            .await?;

            if result.is_none() {
                // Verify that the existing object ID is of the correct type. Even if we're
                // allowing an existing ID, it still must be the correct type.
                let existing_type =
                    sqlx::query_scalar!("SELECT type::text FROM object_ids WHERE object_id=$1", id)
                        .fetch_one(&mut *tx)
                        .await?
                        .unwrap_or_else(String::new);
                if existing_type != object_type {
                    return Err(Error::ObjectIdTypeMismatch {
                        id: *id,
                        wanted: object_type.to_string(),
                        saw: existing_type,
                    });
                }
            }

            *id
        }
        (None, _) => new_object_id(tx, object_type).await?,
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn new_value() {}

    #[test]
    #[ignore]
    fn with_existing_value_of_same_type() {}

    #[test]
    #[ignore]
    fn with_existing_value_of_different_type() {}
}
