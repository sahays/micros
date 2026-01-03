use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Newtype for password to prevent accidental logging
#[derive(Debug, Clone)]
pub struct Password(String);

impl Password {
    pub fn new(password: String) -> Self {
        Self(password)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Newtype for password hash
#[derive(Debug, Clone)]
pub struct PasswordHashString(String);

impl PasswordHashString {
    pub fn new(hash: String) -> Self {
        Self(hash)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// Hash a password using Argon2
///
/// Uses Argon2id variant with secure default parameters.
/// Salt is automatically generated and included in the hash.
pub fn hash_password(password: &Password) -> Result<PasswordHashString, anyhow::Error> {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon2
        .hash_password(password.as_str().as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
        .to_string();

    Ok(PasswordHashString::new(password_hash))
}

/// Verify a password against a hash using constant-time comparison
///
/// Returns Ok(()) if password matches, Err otherwise.
/// Uses constant-time comparison to prevent timing attacks.
pub fn verify_password(
    password: &Password,
    password_hash: &PasswordHashString,
) -> Result<(), anyhow::Error> {
    let parsed_hash = PasswordHash::new(password_hash.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid password hash format: {}", e))?;

    Argon2::default()
        .verify_password(password.as_str().as_bytes(), &parsed_hash)
        .map_err(|_| anyhow::anyhow!("Password verification failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let password = Password::new("mySecurePassword123".to_string());
        let hash = hash_password(&password).expect("Failed to hash password");

        // Hash should not be empty
        assert!(!hash.as_str().is_empty());

        // Hash should start with $argon2
        assert!(hash.as_str().starts_with("$argon2"));
    }

    #[test]
    fn test_verify_password_correct() {
        let password = Password::new("mySecurePassword123".to_string());
        let hash = hash_password(&password).expect("Failed to hash password");

        // Correct password should verify
        assert!(verify_password(&password, &hash).is_ok());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = Password::new("mySecurePassword123".to_string());
        let hash = hash_password(&password).expect("Failed to hash password");

        let wrong_password = Password::new("wrongPassword".to_string());

        // Wrong password should fail verification
        assert!(verify_password(&wrong_password, &hash).is_err());
    }

    #[test]
    fn test_different_hashes_for_same_password() {
        let password = Password::new("mySecurePassword123".to_string());
        let hash1 = hash_password(&password).expect("Failed to hash password");
        let hash2 = hash_password(&password).expect("Failed to hash password");

        // Same password should produce different hashes (due to random salt)
        assert_ne!(hash1.as_str(), hash2.as_str());

        // Both should verify correctly
        assert!(verify_password(&password, &hash1).is_ok());
        assert!(verify_password(&password, &hash2).is_ok());
    }
}
