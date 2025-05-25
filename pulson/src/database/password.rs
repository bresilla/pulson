use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub struct PasswordManager;

impl PasswordManager {
    /// Hash a password using Argon2
    pub fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
        
        Ok(password_hash.to_string())
    }

    /// Verify a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow::anyhow!("Failed to parse password hash: {}", e))?;
        
        let argon2 = Argon2::default();
        
        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Failed to verify password: {}", e)),
        }
    }

    /// Check if a password meets security requirements
    pub fn is_strong_password(password: &str) -> bool {
        let min_length = 8;
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

        password.len() >= min_length && has_uppercase && has_lowercase && has_digit && has_special
    }

    /// Validate password strength and return error message if weak
    pub fn validate_password_strength(password: &str) -> Result<()> {
        if password.len() < 8 {
            return Err(anyhow::anyhow!("Password must be at least 8 characters long"));
        }
        
        if !password.chars().any(|c| c.is_uppercase()) {
            return Err(anyhow::anyhow!("Password must contain at least one uppercase letter"));
        }
        
        if !password.chars().any(|c| c.is_lowercase()) {
            return Err(anyhow::anyhow!("Password must contain at least one lowercase letter"));
        }
        
        if !password.chars().any(|c| c.is_numeric()) {
            return Err(anyhow::anyhow!("Password must contain at least one number"));
        }
        
        if !password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)) {
            return Err(anyhow::anyhow!("Password must contain at least one special character"));
        }
        
        Ok(())
    }

    /// Generate a secure random password
    pub fn generate_secure_password(length: usize) -> String {
        use argon2::password_hash::rand_core::RngCore;
        
        let charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
        let mut rng = OsRng;
        let mut password = String::new();
        
        for _ in 0..length {
            let idx = (rng.next_u32() % charset.len() as u32) as usize;
            password.push(charset.chars().nth(idx).unwrap());
        }
        
        password
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "TestPassword123!";
        let hash = PasswordManager::hash_password(password).unwrap();
        
        assert!(PasswordManager::verify_password(password, &hash).unwrap());
        assert!(!PasswordManager::verify_password("WrongPassword", &hash).unwrap());
    }

    #[test]
    fn test_password_strength_validation() {
        assert!(PasswordManager::is_strong_password("StrongPass123!"));
        assert!(!PasswordManager::is_strong_password("weak"));
        assert!(!PasswordManager::is_strong_password("weakpassword"));
        assert!(!PasswordManager::is_strong_password("WEAKPASSWORD"));
        assert!(!PasswordManager::is_strong_password("WeakPassword"));
        assert!(!PasswordManager::is_strong_password("WeakPass123"));
    }

    #[test]
    fn test_secure_password_generation() {
        let password = PasswordManager::generate_secure_password(16);
        assert_eq!(password.len(), 16);
        assert!(PasswordManager::is_strong_password(&password));
    }

    #[test]
    fn test_password_validation_errors() {
        assert!(PasswordManager::validate_password_strength("short").is_err());
        assert!(PasswordManager::validate_password_strength("nouppercase123!").is_err());
        assert!(PasswordManager::validate_password_strength("NOLOWERCASE123!").is_err());
        assert!(PasswordManager::validate_password_strength("NoNumbers!").is_err());
        assert!(PasswordManager::validate_password_strength("NoSpecialChars123").is_err());
        assert!(PasswordManager::validate_password_strength("ValidPassword123!").is_ok());
    }
}