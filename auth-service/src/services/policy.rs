//! Auth policy validation service.
//!
//! Validates passwords and other auth requirements against organization policies.

use crate::models::AuthPolicy;

/// Errors related to auth policy validation.
#[derive(Debug, Clone)]
pub enum PolicyError {
    /// Password is too short.
    PasswordTooShort {
        min_length: u8,
        actual_length: usize,
    },
    /// Password missing uppercase letter.
    PasswordMissingUppercase,
    /// Password missing number.
    PasswordMissingNumber,
    /// Password missing special character.
    PasswordMissingSpecial,
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::PasswordTooShort {
                min_length,
                actual_length,
            } => {
                write!(
                    f,
                    "Password must be at least {} characters (got {})",
                    min_length, actual_length
                )
            }
            PolicyError::PasswordMissingUppercase => {
                write!(f, "Password must contain at least one uppercase letter")
            }
            PolicyError::PasswordMissingNumber => {
                write!(f, "Password must contain at least one number")
            }
            PolicyError::PasswordMissingSpecial => {
                write!(f, "Password must contain at least one special character")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// Auth policy validation service.
#[derive(Debug, Clone)]
pub struct PolicyService;

impl PolicyService {
    /// Validate a password against the organization's auth policy.
    ///
    /// Returns Ok(()) if the password meets all requirements,
    /// or Err with the first policy violation found.
    pub fn validate_password(password: &str, policy: &AuthPolicy) -> Result<(), PolicyError> {
        // Check minimum length
        if password.len() < policy.password_min_length as usize {
            return Err(PolicyError::PasswordTooShort {
                min_length: policy.password_min_length,
                actual_length: password.len(),
            });
        }

        // Check uppercase requirement
        if policy.password_require_uppercase && !password.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(PolicyError::PasswordMissingUppercase);
        }

        // Check number requirement
        if policy.password_require_number && !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(PolicyError::PasswordMissingNumber);
        }

        // Check special character requirement
        if policy.password_require_special
            && !password.chars().any(|c| {
                // Common special characters
                matches!(
                    c,
                    '!' | '@'
                        | '#'
                        | '$'
                        | '%'
                        | '^'
                        | '&'
                        | '*'
                        | '('
                        | ')'
                        | '-'
                        | '_'
                        | '='
                        | '+'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '|'
                        | '\\'
                        | ';'
                        | ':'
                        | '\''
                        | '"'
                        | ','
                        | '.'
                        | '<'
                        | '>'
                        | '/'
                        | '?'
                        | '`'
                        | '~'
                )
            })
        {
            return Err(PolicyError::PasswordMissingSpecial);
        }

        Ok(())
    }

    /// Validate all password requirements and return all violations.
    ///
    /// Useful for returning all errors to the user at once.
    pub fn validate_password_all(password: &str, policy: &AuthPolicy) -> Vec<PolicyError> {
        let mut errors = Vec::new();

        if password.len() < policy.password_min_length as usize {
            errors.push(PolicyError::PasswordTooShort {
                min_length: policy.password_min_length,
                actual_length: password.len(),
            });
        }

        if policy.password_require_uppercase && !password.chars().any(|c| c.is_ascii_uppercase()) {
            errors.push(PolicyError::PasswordMissingUppercase);
        }

        if policy.password_require_number && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push(PolicyError::PasswordMissingNumber);
        }

        if policy.password_require_special
            && !password.chars().any(|c| {
                matches!(
                    c,
                    '!' | '@'
                        | '#'
                        | '$'
                        | '%'
                        | '^'
                        | '&'
                        | '*'
                        | '('
                        | ')'
                        | '-'
                        | '_'
                        | '='
                        | '+'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '|'
                        | '\\'
                        | ';'
                        | ':'
                        | '\''
                        | '"'
                        | ','
                        | '.'
                        | '<'
                        | '>'
                        | '/'
                        | '?'
                        | '`'
                        | '~'
                )
            })
        {
            errors.push(PolicyError::PasswordMissingSpecial);
        }

        errors
    }

    /// Check if MFA is required for the organization.
    pub fn is_mfa_required(policy: &AuthPolicy) -> bool {
        policy.mfa_required
    }

    /// Get the session timeout in minutes for the organization.
    pub fn session_timeout_minutes(policy: &AuthPolicy) -> u32 {
        policy.session_timeout_minutes
    }

    /// Get the max failed login attempts before lockout.
    pub fn max_failed_attempts(policy: &AuthPolicy) -> u8 {
        policy.max_failed_attempts
    }

    /// Get the lockout duration in minutes.
    pub fn lockout_duration_minutes(policy: &AuthPolicy) -> u32 {
        policy.lockout_duration_minutes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strict_policy() -> AuthPolicy {
        AuthPolicy {
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_number: true,
            password_require_special: true,
            mfa_required: true,
            mfa_allowed_methods: vec!["totp".to_string()],
            session_timeout_minutes: 30,
            max_failed_attempts: 3,
            lockout_duration_minutes: 30,
        }
    }

    fn lenient_policy() -> AuthPolicy {
        AuthPolicy {
            password_min_length: 1,
            password_require_uppercase: false,
            password_require_number: false,
            password_require_special: false,
            mfa_required: false,
            mfa_allowed_methods: vec![],
            session_timeout_minutes: 60,
            max_failed_attempts: 10,
            lockout_duration_minutes: 5,
        }
    }

    #[test]
    fn test_password_too_short() {
        let policy = strict_policy();
        let result = PolicyService::validate_password("Short1!", &policy);
        assert!(matches!(result, Err(PolicyError::PasswordTooShort { .. })));
    }

    #[test]
    fn test_password_missing_uppercase() {
        let policy = strict_policy();
        let result = PolicyService::validate_password("longenoughpassword1!", &policy);
        assert!(matches!(result, Err(PolicyError::PasswordMissingUppercase)));
    }

    #[test]
    fn test_password_missing_number() {
        let policy = strict_policy();
        let result = PolicyService::validate_password("LongEnoughPassword!", &policy);
        assert!(matches!(result, Err(PolicyError::PasswordMissingNumber)));
    }

    #[test]
    fn test_password_missing_special() {
        let policy = strict_policy();
        let result = PolicyService::validate_password("LongEnoughPassword1", &policy);
        assert!(matches!(result, Err(PolicyError::PasswordMissingSpecial)));
    }

    #[test]
    fn test_valid_password_strict() {
        let policy = strict_policy();
        let result = PolicyService::validate_password("LongEnoughP@ss1", &policy);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_password_lenient() {
        let policy = lenient_policy();
        let result = PolicyService::validate_password("simple", &policy);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_all_returns_multiple_errors() {
        let policy = strict_policy();
        let errors = PolicyService::validate_password_all("short", &policy);
        assert_eq!(errors.len(), 4); // Too short, no uppercase, no number, no special
    }
}
