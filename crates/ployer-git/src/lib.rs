use anyhow::{anyhow, Result};
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use std::path::Path;
use tracing::info;

pub struct GitService;

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

impl GitService {
    pub fn new() -> Self {
        Self
    }

    /// Generate SSH key pair (RSA 4096)
    /// Returns (public_key, private_key)
    pub fn generate_deploy_key() -> Result<(String, String)> {
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
        use rsa::{RsaPrivateKey, RsaPublicKey};
        use rand::rngs::OsRng;

        // Generate 4096-bit RSA key
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 4096)
            .map_err(|e| anyhow!("Failed to generate private key: {}", e))?;
        let public_key = RsaPublicKey::from(&private_key);

        // Encode private key as PEM
        let private_pem = private_key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| anyhow!("Failed to encode private key: {}", e))?;

        // Encode public key as OpenSSH format
        let public_ssh = public_key
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| anyhow!("Failed to encode public key: {}", e))?;

        Ok((public_ssh, private_pem.to_string()))
    }

    /// Clone a repository with optional SSH key authentication
    pub fn clone_repo(
        &self,
        url: &str,
        dest: &Path,
        branch: &str,
        private_key: Option<&str>,
    ) -> Result<()> {
        info!("Cloning {} (branch: {}) to {:?}", url, branch, dest);

        let mut callbacks = RemoteCallbacks::new();

        // Set up SSH authentication if private key is provided
        if let Some(key) = private_key {
            let key_owned = key.to_string();
            callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                Cred::ssh_key_from_memory(
                    username_from_url.unwrap_or("git"),
                    None,
                    &key_owned,
                    None,
                )
            });
        }

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.branch(branch);
        builder.fetch_options(fetch_options);

        builder.clone(url, dest)?;
        Ok(())
    }

    /// Pull latest changes from remote
    pub fn pull_latest(&self, repo_path: &Path, branch: &str, private_key: Option<&str>) -> Result<()> {
        info!("Pulling latest changes for branch {} at {:?}", branch, repo_path);

        let repo = Repository::open(repo_path)?;

        // Set up callbacks for authentication
        let mut callbacks = RemoteCallbacks::new();
        if let Some(key) = private_key {
            let key_owned = key.to_string();
            callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                Cred::ssh_key_from_memory(
                    username_from_url.unwrap_or("git"),
                    None,
                    &key_owned,
                    None,
                )
            });
        }

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // Fetch from remote
        let mut remote = repo.find_remote("origin")?;
        remote.fetch(&[branch], Some(&mut fetch_options), None)?;

        // Fast-forward merge
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

        let analysis = repo.merge_analysis(&[&fetch_commit])?;

        if analysis.0.is_up_to_date() {
            info!("Already up to date");
        } else if analysis.0.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", branch);
            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            repo.set_head(&refname)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            info!("Fast-forwarded to latest commit");
        } else {
            return Err(anyhow!("Cannot fast-forward, manual merge required"));
        }

        Ok(())
    }

    /// Get the latest commit information
    pub fn get_latest_commit(&self, repo_path: &Path) -> Result<CommitInfo> {
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;

        // Extract values before commit goes out of scope
        let sha = commit.id().to_string();
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("Unknown").to_string();
        let timestamp = commit.time().seconds();

        Ok(CommitInfo {
            sha,
            message,
            author,
            timestamp,
        })
    }

    /// Checkout a specific branch
    pub fn checkout_branch(&self, repo_path: &Path, branch: &str) -> Result<()> {
        info!("Checking out branch {} at {:?}", branch, repo_path);

        let repo = Repository::open(repo_path)?;

        // Find the branch reference
        let refname = format!("refs/heads/{}", branch);
        let _reference = repo.find_reference(&refname)?;

        // Checkout the branch
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

        Ok(())
    }
}
