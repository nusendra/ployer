use anyhow::Result;
use std::path::Path;
use tracing::info;

pub struct GitService;

impl GitService {
    pub fn new() -> Self {
        Self
    }

    /// Clone a repository to the given path
    pub fn clone_repo(&self, url: &str, dest: &Path, branch: &str) -> Result<()> {
        info!("Cloning {} (branch: {}) to {:?}", url, branch, dest);
        let mut builder = git2::build::RepoBuilder::new();
        builder.branch(branch);
        builder.clone(url, dest)?;
        Ok(())
    }
}
