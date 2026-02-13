use anyhow::Result;
use ployer_core::models::{User, UserRole};
use ployer_db::repositories::{ApiKeyRepository, UserRepository};
use sqlx::SqlitePool;

use super::password::{hash_password, verify_password};
use super::jwt::generate_token;

pub struct AuthService {
    user_repo: UserRepository,
    #[allow(dead_code)]
    api_key_repo: ApiKeyRepository,
}

impl AuthService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            user_repo: UserRepository::new(pool.clone()),
            api_key_repo: ApiKeyRepository::new(pool),
        }
    }

    /// Register a new user. First user automatically becomes admin.
    pub async fn register(&self, email: &str, password: &str, name: &str) -> Result<User> {
        // Check if user already exists
        if self.user_repo.find_by_email(email).await?.is_some() {
            anyhow::bail!("User with this email already exists");
        }

        // Determine role: first user is admin, others are regular users
        let user_count = self.user_repo.count().await?;
        let role = if user_count == 0 {
            UserRole::Admin
        } else {
            UserRole::User
        };

        // Hash password
        let password_hash = hash_password(password)?;

        // Create user
        let user = self.user_repo.create(email, &password_hash, name, role).await?;

        Ok(user)
    }

    /// Login with email and password, returns JWT token
    pub async fn login(&self, email: &str, password: &str, jwt_secret: &str, token_expiry_hours: u64) -> Result<(User, String)> {
        // Find user by email
        let user = self.user_repo
            .find_by_email(email)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid email or password"))?;

        // Verify password
        if !verify_password(password, &user.password_hash)? {
            anyhow::bail!("Invalid email or password");
        }

        // Generate JWT token
        let token = generate_token(
            &user.id,
            &user.email,
            user.role.as_str(),
            jwt_secret,
            token_expiry_hours,
        )?;

        Ok((user, token))
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: &str) -> Result<Option<User>> {
        self.user_repo.find_by_id(user_id).await
    }
}
