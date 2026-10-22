//! Network policy for processing-time HTTP calls (architecture invariant 3).

use std::net::IpAddr;
use std::time::Duration;

use ureq::http::Uri;

use crate::model::ProcessingMode;

/// Error code stored on the meeting when the policy blocks a call.
pub const BLOCKED: &str = "net_policy_blocked";

#[derive(Debug, thiserror::Error)]
#[error("net_policy_blocked")]
pub struct Blocked;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetPolicy {
    pub mode: ProcessingMode,
}

impl NetPolicy {
    /// Call before opening any socket. In `Local` mode only loopback hosts pass.
    pub fn check(&self, url: &str) -> Result<(), Blocked> {
        if self.mode != ProcessingMode::Local {
            return Ok(());
        }
        let uri: Uri = url.parse().map_err(|_| Blocked)?;
        let host = uri.host().ok_or(Blocked)?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let loopback = host.eq_ignore_ascii_case("localhost")
            || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
        if loopback {
            Ok(())
        } else {
            Err(Blocked)
        }
    }
}

/// HTTP agent that never follows redirects: a redirect could leave loopback after `check`.
pub(crate) fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .build()
        .into()
}

/// True when `err` (or anything in its chain) is a policy block.
pub fn is_blocked(err: &anyhow::Error) -> bool {
    err.chain().any(|e| e.is::<Blocked>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mode_allows_only_loopback() {
        let p = NetPolicy {
            mode: ProcessingMode::Local,
        };
        for ok in [
            "http://localhost:11434/v1",
            "http://127.0.0.1/x",
            "http://127.8.9.10:1",
            "http://[::1]:8080/",
            "https://LOCALHOST",
        ] {
            assert!(p.check(ok).is_ok(), "{ok}");
        }
        for bad in [
            "https://api.openai.com/v1",
            "http://10.0.0.1/",
            "http://localhost.evil.com/",
            "http://127.0.0.1.nip.io/",
            "http://[::2]/",
            "not a url",
            "/relative",
        ] {
            assert!(p.check(bad).is_err(), "{bad}");
        }
        for mode in [ProcessingMode::Hybrid, ProcessingMode::Api] {
            assert!(NetPolicy { mode }
                .check("https://openrouter.ai/api/v1/chat/completions")
                .is_ok());
        }
    }
}
