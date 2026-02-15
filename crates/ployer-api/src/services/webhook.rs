use anyhow::{Result, anyhow};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};

type HmacSha256 = Hmac<Sha256>;

/// Parsed webhook payload with standardized fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub branch: String,
    pub commit_sha: String,
    pub commit_message: String,
    pub author: String,
    pub repository_url: String,
}

/// GitHub push event payload (subset of fields we care about)
#[derive(Debug, Deserialize)]
struct GitHubPushEvent {
    #[serde(rename = "ref")]
    git_ref: String,
    head_commit: GitHubCommit,
    repository: GitHubRepository,
}

#[derive(Debug, Deserialize)]
struct GitHubCommit {
    id: String,
    message: String,
    author: GitHubAuthor,
}

#[derive(Debug, Deserialize)]
struct GitHubAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRepository {
    clone_url: String,
}

/// GitLab push event payload (subset of fields we care about)
#[derive(Debug, Deserialize)]
struct GitLabPushEvent {
    #[serde(rename = "ref")]
    git_ref: String,
    checkout_sha: String,
    commits: Vec<GitLabCommit>,
    repository: GitLabRepository,
}

#[derive(Debug, Deserialize)]
struct GitLabCommit {
    message: String,
    author: GitLabAuthor,
}

#[derive(Debug, Deserialize)]
struct GitLabAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitLabRepository {
    git_ssh_url: String,
}

/// Verify GitHub webhook signature (X-Hub-Signature-256 header)
pub fn verify_github_signature(secret: &str, payload: &[u8], signature: &str) -> Result<()> {
    // GitHub signature format: "sha256=<hex>"
    let expected_sig = signature
        .strip_prefix("sha256=")
        .ok_or_else(|| anyhow!("Invalid GitHub signature format"))?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow!("Invalid HMAC key: {}", e))?;
    mac.update(payload);

    let result = mac.finalize();
    let computed_sig = hex::encode(result.into_bytes());

    if computed_sig != expected_sig {
        return Err(anyhow!("GitHub signature verification failed"));
    }

    Ok(())
}

/// Verify GitLab webhook signature (X-Gitlab-Token header)
pub fn verify_gitlab_signature(secret: &str, token: &str) -> Result<()> {
    if secret != token {
        return Err(anyhow!("GitLab token verification failed"));
    }
    Ok(())
}

/// Parse GitHub push event payload
pub fn parse_github_push(payload: &[u8]) -> Result<WebhookPayload> {
    let event: GitHubPushEvent = serde_json::from_slice(payload)
        .map_err(|e| anyhow!("Failed to parse GitHub payload: {}", e))?;

    // Extract branch name from ref (refs/heads/main -> main)
    let branch = event.git_ref
        .strip_prefix("refs/heads/")
        .unwrap_or(&event.git_ref)
        .to_string();

    Ok(WebhookPayload {
        branch,
        commit_sha: event.head_commit.id,
        commit_message: event.head_commit.message,
        author: event.head_commit.author.name,
        repository_url: event.repository.clone_url,
    })
}

/// Parse GitLab push event payload
pub fn parse_gitlab_push(payload: &[u8]) -> Result<WebhookPayload> {
    let event: GitLabPushEvent = serde_json::from_slice(payload)
        .map_err(|e| anyhow!("Failed to parse GitLab payload: {}", e))?;

    // Extract branch name from ref (refs/heads/main -> main)
    let branch = event.git_ref
        .strip_prefix("refs/heads/")
        .unwrap_or(&event.git_ref)
        .to_string();

    // Get the latest commit (GitLab sends array, we want the newest)
    let latest_commit = event.commits
        .first()
        .ok_or_else(|| anyhow!("No commits in GitLab push event"))?;

    Ok(WebhookPayload {
        branch,
        commit_sha: event.checkout_sha,
        commit_message: latest_commit.message.clone(),
        author: latest_commit.author.name.clone(),
        repository_url: event.repository.git_ssh_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_signature_verification() {
        let secret = "my-secret";
        let payload = b"{\"test\":\"data\"}";

        // Compute expected signature
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let result = mac.finalize();
        let signature = format!("sha256={}", hex::encode(result.into_bytes()));

        assert!(verify_github_signature(secret, payload, &signature).is_ok());
        assert!(verify_github_signature("wrong-secret", payload, &signature).is_err());
    }

    #[test]
    fn test_gitlab_signature_verification() {
        let secret = "my-token";
        assert!(verify_gitlab_signature(secret, "my-token").is_ok());
        assert!(verify_gitlab_signature(secret, "wrong-token").is_err());
    }

    #[test]
    fn test_parse_github_push() {
        let payload = r#"{
            "ref": "refs/heads/main",
            "head_commit": {
                "id": "abc123",
                "message": "Fix bug",
                "author": {"name": "John Doe"}
            },
            "repository": {
                "clone_url": "https://github.com/user/repo.git"
            }
        }"#;

        let result = parse_github_push(payload.as_bytes()).unwrap();
        assert_eq!(result.branch, "main");
        assert_eq!(result.commit_sha, "abc123");
        assert_eq!(result.commit_message, "Fix bug");
        assert_eq!(result.author, "John Doe");
    }

    #[test]
    fn test_parse_gitlab_push() {
        let payload = r#"{
            "ref": "refs/heads/develop",
            "checkout_sha": "def456",
            "commits": [
                {
                    "message": "Add feature",
                    "author": {"name": "Jane Smith"}
                }
            ],
            "repository": {
                "git_ssh_url": "git@gitlab.com:user/repo.git"
            }
        }"#;

        let result = parse_gitlab_push(payload.as_bytes()).unwrap();
        assert_eq!(result.branch, "develop");
        assert_eq!(result.commit_sha, "def456");
        assert_eq!(result.commit_message, "Add feature");
        assert_eq!(result.author, "Jane Smith");
    }
}
