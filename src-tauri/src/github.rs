use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const UA: &str = "russus-launcher";

#[derive(Debug, Deserialize)]
struct Repo {
    name: String,
    #[serde(default)]
    description: Option<String>,
    html_url: String,
    #[serde(default)]
    fork: bool,
    #[serde(default)]
    archived: bool,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<Asset>,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseInfo {
    pub tag: String,
    pub published_at: Option<String>,
    pub asset_url: Option<String>,
    pub asset_name: Option<String>,
}

pub struct GitHub {
    client: reqwest::Client,
    token: Option<String>,
}

impl GitHub {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(UA)
                .build()
                .unwrap_or_default(),
            token,
        }
    }

    fn req(&self, url: &str) -> reqwest::RequestBuilder {
        let mut r = self
            .client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = &self.token {
            r = r.bearer_auth(t);
        }
        r
    }

    /// List non-fork, non-archived repos owned by `user`, excluding helper repos.
    pub async fn list_repos(&self, user: &str) -> Result<Vec<RepoMeta>, String> {
        let mut out = Vec::new();
        for page in 1..=5 {
            let url = format!(
                "https://api.github.com/users/{user}/repos?per_page=100&page={page}&type=owner&sort=updated"
            );
            let resp = self.req(&url).send().await.map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                return Err(format!("GitHub API: {} ({})", resp.status(), user));
            }
            let repos: Vec<Repo> = resp.json().await.map_err(|e| e.to_string())?;
            let n = repos.len();
            for r in repos {
                if r.fork || r.archived {
                    continue;
                }
                if r.name.ends_with("-dist") || r.name == "dotfiles" {
                    continue;
                }
                out.push(RepoMeta {
                    name: r.name,
                    description: r.description.unwrap_or_default(),
                    html_url: r.html_url,
                });
            }
            if n < 100 {
                break;
            }
        }
        Ok(out)
    }

    /// Latest non-draft release for a repo, with the asset matching the current platform.
    pub async fn latest_release(&self, user: &str, repo: &str) -> Result<Option<ReleaseInfo>, String> {
        let url = format!("https://api.github.com/repos/{user}/{repo}/releases?per_page=10");
        let resp = self.req(&url).send().await.map_err(|e| e.to_string())?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!("GitHub API: {} ({repo})", resp.status()));
        }
        let releases: Vec<Release> = resp.json().await.map_err(|e| e.to_string())?;
        let rel = releases
            .into_iter()
            .find(|r| !r.draft && !r.prerelease)
            .or_else(|| None);
        let Some(rel) = rel else { return Ok(None) };

        let asset = pick_platform_asset(&rel.assets);
        Ok(Some(ReleaseInfo {
            tag: rel.tag_name,
            published_at: rel.published_at,
            asset_url: asset.as_ref().map(|a| a.browser_download_url.clone()),
            asset_name: asset.map(|a| a.name.clone()),
        }))
    }
}

pub struct RepoMeta {
    pub name: String,
    pub description: String,
    pub html_url: String,
}

/// Pick the right installable asset for the platform we are running on.
fn pick_platform_asset(assets: &[Asset]) -> Option<&Asset> {
    #[cfg(target_os = "linux")]
    {
        // Arch package first; the launcher is built for Arch (.zst).
        return assets
            .iter()
            .find(|a| a.name.ends_with(".pkg.tar.zst"));
    }
    #[cfg(target_os = "macos")]
    {
        let arch = if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x64"
        };
        // Prefer an arch-specific dmg, else any dmg.
        return assets
            .iter()
            .find(|a| a.name.ends_with(".dmg") && a.name.contains(arch))
            .or_else(|| assets.iter().find(|a| a.name.ends_with(".dmg")));
    }
    #[allow(unreachable_code)]
    {
        let _ = assets;
        None
    }
}

/// Strip a leading `v` and any pre-release/build suffix for comparison.
pub fn normalize_version(v: &str) -> String {
    v.trim().trim_start_matches(['v', 'V']).to_string()
}

/// Compare two dotted version strings numerically (1.10 > 1.9).
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let pa = parse_parts(a);
    let pb = parse_parts(b);
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

fn parse_parts(v: &str) -> Vec<u64> {
    normalize_version(v)
        .split(['.', '-', '+'])
        .map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .unwrap_or(0)
        })
        .collect()
}

/// Derive the pacman package name from an Arch asset filename.
/// e.g. "pdf-accessibility-studio-1.9.0-1-x86_64.pkg.tar.zst" -> "pdf-accessibility-studio"
#[cfg(target_os = "linux")]
pub fn pkgname_from_asset(asset: &str) -> Option<String> {
    let stem = asset.strip_suffix(".pkg.tar.zst")?;
    let parts: Vec<&str> = stem.split('-').collect();
    if parts.len() < 4 {
        return None;
    }
    Some(parts[..parts.len() - 3].join("-"))
}
