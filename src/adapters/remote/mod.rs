//! Cloud-distributed check fetching
//!
//! Fetches the org's effective check set for this repo from noslop cloud
//! (`GET /api/v1/repo/checks`, bearer repo token) with an on-disk cache.
//!
//! FAIL-OPEN INVARIANT: like uploads, the fetch is telemetry-grade — a
//! cloud outage degrades to cached checks, then to repo-local checks with
//! a warning. It never blocks a commit and never returns an error to the
//! gate path.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::adapters::toml::RemoteConfig;

/// Gitignored per-clone cache of the last fetched check set
pub const CACHE_FILE: &str = ".noslop/remote-checks.json";

/// How long a cached set is fresh (seconds)
const CACHE_TTL_SECS: u64 = 300;

/// Default env var holding the repo token
const DEFAULT_TOKEN_ENV: &str = "NOSLOP_CLOUD_TOKEN";

/// One check as distributed by the cloud (mirrors the API contract)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCheck {
    /// Stable check id (registry slug)
    pub id: String,
    /// File glob the check applies to
    pub target: String,
    /// The ask
    pub message: String,
    /// Severity: info, warn, block
    pub severity: String,
    /// Lifecycle state: "monitor" evaluates silently; "advisory"/"enforce" gate
    pub state: String,
    /// Named owner of the check
    pub owner: String,
    /// Named, expiring exceptions
    #[serde(default)]
    pub bypasses: Vec<RemoteBypass>,
}

/// A named, expiring exception for one actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteBypass {
    /// Actor the exception applies to
    pub grantee: String,
    /// Why the exception exists
    pub reason: String,
    /// RFC3339 expiry — bypasses are never permanent
    pub expires_at: String,
}

impl RemoteBypass {
    /// Whether this bypass currently exempts the given actor
    #[must_use]
    pub fn exempts(&self, actor: &str, now: &chrono::DateTime<chrono::Utc>) -> bool {
        if self.grantee != actor {
            return false;
        }
        chrono::DateTime::parse_from_rfc3339(&self.expires_at)
            .is_ok_and(|expiry| expiry.with_timezone(&chrono::Utc) > *now)
    }
}

/// The versioned effective check set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCheckSet {
    /// Content hash of the set; echoed into envelopes for fidelity
    pub check_set_version: String,
    /// The effective checks for this repo
    pub checks: Vec<RemoteCheck>,
}

/// A check set plus how stale it is. The gate is fail-open, so a run may
/// legitimately proceed on cached rules — the age travels in the envelope
/// so the cloud can tell a fresh ruleset from a stale one.
#[derive(Debug, Clone)]
pub struct FetchedCheckSet {
    /// The effective check set
    pub set: RemoteCheckSet,
    /// Seconds since the set was fetched from the cloud (0 = this run)
    pub age_seconds: u64,
}

#[derive(Serialize, Deserialize)]
struct CachedSet {
    fetched_at: u64,
    set: RemoteCheckSet,
}

impl CachedSet {
    fn into_fetched(self) -> FetchedCheckSet {
        FetchedCheckSet {
            age_seconds: now_unix().saturating_sub(self.fetched_at),
            set: self.set,
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn read_cache() -> Option<CachedSet> {
    let content = fs::read_to_string(CACHE_FILE).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_cache(set: &RemoteCheckSet) {
    let cached = CachedSet {
        fetched_at: now_unix(),
        set: set.clone(),
    };
    if let Ok(json) = serde_json::to_string(&cached) {
        let _ =
            fs::create_dir_all(Path::new(CACHE_FILE).parent().unwrap_or_else(|| Path::new(".")));
        let _ = fs::write(CACHE_FILE, json);
    }
}

fn fetch(url: &str, token: &str) -> anyhow::Result<RemoteCheckSet> {
    let endpoint = format!("{}/api/v1/repo/checks", url.trim_end_matches('/'));
    let set: RemoteCheckSet = ureq::get(&endpoint)
        .set("Authorization", &format!("Bearer {token}"))
        .timeout(std::time::Duration::from_secs(5))
        .call()?
        .into_json()?;
    Ok(set)
}

/// Load the org's check set for this clone, or `None` when the repo has no
/// remote binding or nothing is reachable. Warnings go to stderr; errors
/// never propagate (fail-open).
#[must_use]
pub fn load_remote_checks(config: &RemoteConfig) -> Option<FetchedCheckSet> {
    let url = config.url.as_deref()?;
    let token_env = config.token_env.as_deref().unwrap_or(DEFAULT_TOKEN_ENV);
    let Ok(token) = std::env::var(token_env) else {
        eprintln!(
            "noslop: [remote] configured but ${token_env} is not set; using local checks only"
        );
        return None;
    };

    if let Some(cached) = read_cache()
        && now_unix().saturating_sub(cached.fetched_at) < CACHE_TTL_SECS
    {
        return Some(cached.into_fetched());
    }

    match fetch(url, &token) {
        Ok(set) => {
            write_cache(&set);
            Some(FetchedCheckSet {
                set,
                age_seconds: 0,
            })
        },
        Err(err) => read_cache().map_or_else(
            || {
                eprintln!("noslop: remote check fetch failed ({err}); using local checks only");
                None
            },
            |cached| {
                eprintln!("noslop: remote check fetch failed ({err}); using cached set");
                Some(cached.into_fetched())
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_exempts_only_matching_unexpired_actor() {
        let now = chrono::Utc::now();
        let bypass = RemoteBypass {
            grantee: "release-bot".into(),
            reason: "hotfix".into(),
            expires_at: (now + chrono::Duration::hours(1)).to_rfc3339(),
        };
        assert!(bypass.exempts("release-bot", &now));
        assert!(!bypass.exempts("claude-code", &now));

        let expired = RemoteBypass {
            expires_at: (now - chrono::Duration::hours(1)).to_rfc3339(),
            ..bypass
        };
        assert!(!expired.exempts("release-bot", &now));

        let garbage = RemoteBypass {
            expires_at: "not-a-date".into(),
            ..expired
        };
        assert!(!garbage.exempts("release-bot", &now));
    }

    #[test]
    fn cached_set_reports_age_since_fetch() {
        let cached = CachedSet {
            fetched_at: now_unix() - 120,
            set: RemoteCheckSet {
                check_set_version: "v1".into(),
                checks: vec![],
            },
        };
        let fetched = cached.into_fetched();
        assert!((120..125).contains(&fetched.age_seconds));

        // A clock that runs backwards must not underflow
        let future = CachedSet {
            fetched_at: now_unix() + 999,
            set: fetched.set,
        };
        assert_eq!(future.into_fetched().age_seconds, 0);
    }

    #[test]
    fn check_set_round_trips() {
        let json = r#"{"check_set_version":"abc123","checks":[{"id":"ORG-1","target":"src/**","message":"ask","severity":"block","state":"monitor","owner":"saumil","bypasses":[]}]}"#;
        let set: RemoteCheckSet = serde_json::from_str(json).unwrap();
        assert_eq!(set.checks[0].state, "monitor");
        assert_eq!(
            serde_json::from_str::<RemoteCheckSet>(&serde_json::to_string(&set).unwrap())
                .unwrap()
                .check_set_version,
            "abc123"
        );
    }
}
