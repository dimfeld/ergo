use itertools::Itertools;
use std::env;

use crate::error::Error;

pub type PostgresPool = sqlx::PgPool;

pub struct PostgresAuth {
    pub username: String,
    pub password: String,
}

impl PostgresAuth {
    pub fn from_env(database_role_env_name: &str, default_username: &str) -> Result<Self, Error> {
        let db_username_env = format!("DATABASE_ROLE_{}_USERNAME", database_role_env_name);
        let db_password_env = format!("DATABASE_ROLE_{}_PASSWORD", database_role_env_name);
        let username = env::var(&db_username_env).unwrap_or_else(|_| default_username.to_string());
        let password = env::var(&db_password_env).map_err(|_| {
            Error::ConfigError(format!("Failed to read password from {}", db_password_env))
        })?;

        Ok(PostgresAuth { username, password })
    }
}

pub fn sql_insert_parameters<const NCOL: usize>(num_rows: usize) -> String {
    (0..num_rows)
        .into_iter()
        .map(|i| {
            let base = i * NCOL + 1;
            let mut output = String::with_capacity(2 + NCOL * 4);

            output.push('(');
            output.push('$');
            output.push_str(base.to_string().as_str());
            for i in 1..NCOL {
                output.push_str(",$");
                output.push_str((base + i).to_string().as_str());
            }
            output.push(')');

            output
        })
        .join(",\n")
}

#[cfg(test)]
mod tests {
    use super::sql_insert_parameters as sip;

    #[test]
    fn sql_insert_parameters() {
        assert_eq!(
            sip::<2>(3),
            r##"($1,$2),
($3,$4),
($5,$6)"##
        );
    }
}
