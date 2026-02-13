pub mod password;
pub mod jwt;
pub mod middleware;
pub mod service;
pub mod helpers;

pub use password::hash_password;
pub use jwt::{generate_token, validate_token, Claims};
pub use middleware::auth_middleware;
pub use service::AuthService;
pub use helpers::extract_user_id;
