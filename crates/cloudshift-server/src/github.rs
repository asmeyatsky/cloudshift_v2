//! Fetch public GitHub repos as zip and extract code files (same filters as UI import).

use anyhow::{anyhow, Context};
use serde::Serialize;
use std::io::{Cursor, Read};
use tracing::warn;

pub const MAX_ZIP_BYTES: usize = 25 * 1024 * 1024;
pub const MAX_REPO_FILES: usize = 80;
pub const MAX_FILE_BYTES: usize = 900_000;

#[derive(Serialize, Clone)]
pub struct GithubRepoFile {
    pub path: String,
    pub source: String,
    pub language: String,
}

#[derive(Serialize)]
pub struct GithubRepoResponse {
    pub files: Vec<GithubRepoFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_ref: Option<String>,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn github_headers(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    let mut req = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "CloudShift-Server/1");
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        let t = t.trim();
        if !t.is_empty() {
            req = req.header("Authorization", format!("Bearer {t}"));
        }
    }
    req
}

/// Returns (owner, repo, ref from /tree/branch if present).
pub fn parse_github_repo_url(input: &str) -> anyhow::Result<(String, String, Option<String>)> {
    let s = input.trim();

    if let Some(rest) = s.strip_prefix("git@github.com:") {
        let rest = rest.trim_end_matches(".git").trim();
        let mut parts = rest.split('/').filter(|p| !p.is_empty());
        let owner = parts
            .next()
            .ok_or_else(|| anyhow!("Expected git@github.com:owner/repo"))?
            .to_string();
        let repo = parts
            .next()
            .ok_or_else(|| anyhow!("Expected git@github.com:owner/repo"))?
            .to_string();
        return Ok((owner, repo, None));
    }

    let s = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s);
    let s = s.strip_prefix("www.").unwrap_or(s);

    if !s.starts_with("github.com/") {
        anyhow::bail!("Use a GitHub URL like https://github.com/owner/repo");
    }

    let path = &s["github.com/".len()..];
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        anyhow::bail!("Expected github.com/owner/repo");
    }

    let owner = parts[0].to_string();
    let repo = parts[1].trim_end_matches(".git").to_string();

    let tree_ref = if parts.len() >= 4 && parts[2] == "tree" {
        Some(parts[3].to_string())
    } else {
        None
    };

    Ok((owner, repo, tree_ref))
}

pub async fn fetch_default_branch(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
) -> anyhow::Result<String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let res = github_headers(client, &url)
        .send()
        .await
        .context("GitHub API request failed")?;
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!(
            "Repository not found or private (set GITHUB_TOKEN on the server for private repos)"
        );
    }
    if !res.status().is_success() {
        anyhow::bail!("GitHub API returned {}", res.status());
    }
    let v: serde_json::Value = res.json().await?;
    v.get("default_branch")
        .and_then(|b| b.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Could not read default branch"))
}

pub async fn download_repo_zip(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    git_ref: &str,
) -> anyhow::Result<Vec<u8>> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/zipball/{git_ref}");
    let res = github_headers(client, &url)
        .send()
        .await
        .context("Download failed")?;
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("Branch or tag not found, or repo is private");
    }
    if !res.status().is_success() {
        anyhow::bail!("GitHub returned {} while downloading archive", res.status());
    }
    let bytes = res.bytes().await?;
    if bytes.len() > MAX_ZIP_BYTES {
        anyhow::bail!("Archive exceeds {} MB limit", MAX_ZIP_BYTES / (1024 * 1024));
    }
    Ok(bytes.to_vec())
}

fn should_skip_path(path: &str) -> bool {
    let p = path.to_lowercase();
    if p.contains("node_modules/")
        || p.contains("/.git/")
        || p.contains("__pycache__")
        || p.contains(".venv/")
        || p.contains("/venv/")
        || p.contains("/dist/")
        || p.contains("/target/")
        || p.contains("/.next/")
        || p.contains("/vendor/")
        || p.contains("/build/")
    {
        return true;
    }
    path.split('/').any(|seg| seg.starts_with('.'))
}

fn guess_language(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    if lower.ends_with("dockerfile") || path.rsplit('/').next() == Some("Dockerfile") {
        return Some("dockerfile");
    }
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_lowercase())?;
    Some(match ext.as_str() {
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "java" => "java",
        "go" => "go",
        "tf" | "hcl" => "hcl",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        _ => return None,
    })
}

pub fn extract_repo_files(zip_bytes: &[u8]) -> anyhow::Result<(Vec<GithubRepoFile>, bool)> {
    let cursor = Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| anyhow!("Invalid zip archive: {e}"))?;

    let mut root_prefix = String::new();
    for i in 0..archive.len() {
        let name = archive.by_index(i).map(|f| f.name().to_string())?;
        if let Some(idx) = name.find('/') {
            root_prefix = name[..idx + 1].to_string();
            break;
        }
    }

    let mut out = Vec::new();
    let mut truncated = false;

    for i in 0..archive.len() {
        if out.len() >= MAX_REPO_FILES {
            truncated = true;
            break;
        }
        let mut file = archive.by_index(i)?;
        if file.is_dir() {
            continue;
        }
        let full = file.name().to_string();
        let rel = if !root_prefix.is_empty() && full.starts_with(&root_prefix) {
            full[root_prefix.len()..].to_string()
        } else {
            full.clone()
        };
        if should_skip_path(&rel) {
            continue;
        }
        let Some(lang) = guess_language(&rel) else {
            continue;
        };
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        if buf.len() > MAX_FILE_BYTES {
            continue;
        }
        if buf.iter().take(4096.min(buf.len())).any(|&b| b == 0) {
            continue;
        }
        let Ok(text) = std::str::from_utf8(&buf) else {
            continue;
        };
        if !text.trim().is_empty() {
            out.push(GithubRepoFile {
                path: rel,
                source: text.to_string(),
                language: lang.to_string(),
            });
        }
    }

    Ok((out, truncated))
}

pub async fn import_github_repo(
    client: &reqwest::Client,
    url: &str,
    ref_override: Option<&str>,
) -> GithubRepoResponse {
    let (owner, repo, tree_ref) = match parse_github_repo_url(url) {
        Ok(x) => x,
        Err(e) => {
            return GithubRepoResponse {
                files: vec![],
                resolved_ref: None,
                truncated: false,
                error: Some(e.to_string()),
            };
        }
    };

    let git_ref = match ref_override.map(str::trim).filter(|s| !s.is_empty()) {
        Some(r) => r.to_string(),
        None => match tree_ref {
            Some(r) => r,
            None => match fetch_default_branch(client, &owner, &repo).await {
                Ok(b) => b,
                Err(e) => {
                    warn!("fetch_default_branch failed for {owner}/{repo}: {e:#}");
                    return GithubRepoResponse {
                        files: vec![],
                        resolved_ref: None,
                        truncated: false,
                        error: Some(
                            "Could not access repository — check URL and permissions".to_string(),
                        ),
                    };
                }
            },
        },
    };

    let zip_bytes = match download_repo_zip(client, &owner, &repo, &git_ref).await {
        Ok(z) => z,
        Err(e) => {
            warn!("download_repo_zip failed for {owner}/{repo}@{git_ref}: {e:#}");
            return GithubRepoResponse {
                files: vec![],
                resolved_ref: Some(git_ref),
                truncated: false,
                error: Some("Failed to download repository archive".to_string()),
            };
        }
    };

    let (files, truncated) = match extract_repo_files(&zip_bytes) {
        Ok(x) => x,
        Err(e) => {
            warn!("extract_repo_files failed for {owner}/{repo}@{git_ref}: {e:#}");
            return GithubRepoResponse {
                files: vec![],
                resolved_ref: Some(git_ref),
                truncated: false,
                error: Some("Failed to process repository archive".to_string()),
            };
        }
    };

    let error = if files.is_empty() {
        Some(
            "No supported code files found in this repo (Python, TS, JS, Java, Go, HCL, YAML, JSON, Dockerfile)."
                .to_string(),
        )
    } else {
        None
    };

    GithubRepoResponse {
        files,
        resolved_ref: Some(git_ref),
        truncated,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_github_repo_url;

    #[test]
    fn parse_https() {
        let (o, r, tr) = parse_github_repo_url("https://github.com/microsoft/vscode").unwrap();
        assert_eq!(o, "microsoft");
        assert_eq!(r, "vscode");
        assert!(tr.is_none());
    }

    #[test]
    fn parse_tree_ref() {
        let (o, r, tr) = parse_github_repo_url("https://github.com/foo/bar/tree/develop").unwrap();
        assert_eq!(o, "foo");
        assert_eq!(r, "bar");
        assert_eq!(tr.as_deref(), Some("develop"));
    }

    #[test]
    fn parse_ssh() {
        let (o, r, _) = parse_github_repo_url("git@github.com:org/repo.git").unwrap();
        assert_eq!(o, "org");
        assert_eq!(r, "repo");
    }
}
