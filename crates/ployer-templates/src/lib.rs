pub mod error;
pub mod schema;
pub mod registry;
pub mod render;

pub use error::TemplateError;
pub use registry::{Registry, RegistryConfig};
pub use render::render;
pub use schema::{Input, IndexEntry, RegistryIndex, Template};
