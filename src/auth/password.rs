use crate::error::{Error, Result};
use argon2::{
    password_hash::{Output, PasswordHash, PasswordHasher, PasswordVerifier, Salt, SaltString},
    Argon2, Params, Version,
};
use uuid::Uuid;

pub fn new_hash(password: &str) -> Result<String> {
    let salt = uuid::Uuid::new_v4();
    hash_password(password, &salt)
}

fn hash_password(password: &str, salt: &Uuid) -> Result<String> {
    let saltstring = SaltString::b64_encode(salt.as_bytes())
        .map_err(|e| Error::PasswordHasherError(e.to_string()))?;

    let params = Params {
        m_cost: 15360,
        t_cost: 2,
        p_cost: 1,
        ..Default::default()
    };

    let hash = Argon2::default()
        .hash_password(password.as_bytes(), None, params, saltstring.as_salt())
        .map_err(|e| Error::PasswordHasherError(e.to_string()))?;

    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash_str: &str) -> Result<()> {
    let hash =
        PasswordHash::new(hash_str).map_err(|e| Error::PasswordHasherError(e.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .map_err(|_| Error::AuthenticationError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;

    #[test]
    fn good_password() -> Result<()> {
        let hash = new_hash("abcdef")?;
        verify_password("abcdef", &hash)
    }

    #[test]
    fn bad_password() -> Result<()> {
        let hash = new_hash("abcdef")?;
        verify_password("abcdefg", &hash).expect_err("non-matching password");
        Ok(())
    }

    #[test]
    fn unique_password_salt() {
        let p1 = new_hash("abc").unwrap();
        let p2 = new_hash("abc").unwrap();
        assert_ne!(p1, p2);
    }
}
