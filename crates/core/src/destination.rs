//! Outgoing destinations (spec section 15). Not subject to `NetPolicy`: sending is
//! user-initiated after review, outside processing.

use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{bail, Result};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::model::{Destinations, MeetingDoc};
use crate::provider::Destination;

pub struct FolderDestination {
    pub path: PathBuf,
}

impl Destination for FolderDestination {
    fn name(&self) -> &str {
        "folder"
    }

    fn send(&self, doc: &MeetingDoc) -> Result<()> {
        crate::export::write_folder(doc, &self.path)
    }
}

pub struct WebhookDestination {
    pub url: String,
    pub secret: Option<String>,
    /// Delay before the 2nd attempt; doubles after each failure.
    pub retry_base: Duration,
    pub attempts: u32,
}

impl WebhookDestination {
    pub fn new(url: String, secret: Option<String>) -> Self {
        Self {
            url,
            secret,
            retry_base: Duration::from_secs(1),
            attempts: 4,
        }
    }
}

impl Destination for WebhookDestination {
    fn name(&self) -> &str {
        "webhook"
    }

    fn send(&self, doc: &MeetingDoc) -> Result<()> {
        let body = serde_json::to_vec(doc)?;
        let agent = crate::net::agent(Duration::from_secs(30));
        let mut last = String::new();
        for attempt in 0..self.attempts {
            if attempt > 0 {
                sleep(self.retry_base * 2u32.pow(attempt - 1));
            }
            let mut req = agent
                .post(&self.url)
                .header("Content-Type", "application/json")
                .header("Idempotency-Key", doc.meeting.id.to_string());
            if let Some(secret) = &self.secret {
                req = req.header("X-Signature", format!("sha256={}", sign(secret, &body)));
            }
            match req.send(&body[..]) {
                Ok(r) if r.status().is_success() => return Ok(()),
                Ok(r) => last = format!("http_{}", r.status().as_u16()),
                Err(ureq::Error::StatusCode(code))
                    if (400..500).contains(&code) && code != 408 && code != 429 =>
                {
                    bail!("http_{code}")
                }
                Err(ureq::Error::StatusCode(code)) => last = format!("http_{code}"),
                Err(_) => last = "transport".into(),
            }
            log::warn!("webhook attempt {} failed: {last}", attempt + 1);
        }
        bail!("webhook_failed:{last}")
    }
}

/// Hex HMAC-SHA256 of `body` keyed by `secret`.
pub fn sign(secret: &str, body: &[u8]) -> String {
    // HMAC accepts keys of any length, so this cannot fail.
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(body);
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Enabled destinations from settings. The webhook secret comes from the OS keyring.
pub fn from_settings(
    d: &Destinations,
    webhook_secret: Option<String>,
) -> Vec<Box<dyn Destination>> {
    let mut out: Vec<Box<dyn Destination>> = Vec::new();
    if d.folder.enabled && !d.folder.path.is_empty() {
        out.push(Box::new(FolderDestination {
            path: d.folder.path.clone().into(),
        }));
    }
    if d.webhook.enabled && !d.webhook.url.is_empty() {
        out.push(Box::new(WebhookDestination::new(
            d.webhook.url.clone(),
            webhook_secret,
        )));
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    #[test]
    fn signature_matches_rfc4231_vector() {
        assert_eq!(
            sign("Jefe", b"what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    /// Minimal HTTP server: answers each request with the next status and
    /// reports (headers, body) of every request received.
    pub(crate) fn serve(
        statuses: Vec<(u16, String)>,
    ) -> (String, mpsc::Receiver<(Vec<String>, String)>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for (status, reply) in statuses {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                let mut reader = BufReader::new(stream);
                let mut headers = Vec::new();
                let mut len = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let line = line.trim_end().to_string();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                    headers.push(line);
                }
                let mut body = vec![0; len];
                reader.read_exact(&mut body).unwrap();
                tx.send((headers, String::from_utf8_lossy(&body).into_owned()))
                    .unwrap();
                let resp = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
                reader.get_mut().write_all(resp.as_bytes()).unwrap();
            }
        });
        (url, rx)
    }

    fn hook(url: String) -> WebhookDestination {
        WebhookDestination {
            url,
            secret: Some("s3cret".into()),
            retry_base: Duration::from_millis(1),
            attempts: 4,
        }
    }

    #[test]
    fn webhook_retries_then_succeeds_with_signed_headers() {
        let doc = crate::export::sample_doc();
        let (url, rx) = serve(vec![
            (500, String::new()),
            (429, String::new()),
            (200, String::new()),
        ]);
        hook(url).send(&doc).unwrap();
        let reqs: Vec<_> = rx.try_iter().collect();
        assert_eq!(reqs.len(), 3);
        let (headers, body) = &reqs[2];
        let has = |h: &str| headers.iter().any(|l| l.eq_ignore_ascii_case(h));
        assert!(has(&format!(
            "x-signature: sha256={}",
            sign("s3cret", body.as_bytes())
        )));
        assert!(has(&format!("idempotency-key: {}", doc.meeting.id)));
        assert!(has("content-type: application/json"));
        assert_eq!(serde_json::from_str::<MeetingDoc>(body).unwrap(), doc);
    }

    #[test]
    fn webhook_test_single_attempt() {
        let doc = crate::export::sample_doc();
        let (url, rx) = serve(vec![(500, String::new()); 2]);
        let hook = WebhookDestination { attempts: 1, ..hook(url) };
        assert_eq!(hook.send(&doc).unwrap_err().to_string(), "webhook_failed:http_500");
        let reqs: Vec<_> = rx.try_iter().collect();
        assert_eq!(reqs.len(), 1);
        let has = |h: &str| reqs[0].0.iter().any(|l| l.eq_ignore_ascii_case(h));
        assert!(has(&format!("x-signature: sha256={}", sign("s3cret", reqs[0].1.as_bytes()))));
    }

    #[test]
    fn webhook_gives_up_after_four_attempts_and_on_4xx() {
        let doc = crate::export::sample_doc();
        let (url, rx) = serve(vec![(503, String::new()); 5]);
        assert!(hook(url).send(&doc).is_err());
        assert_eq!(rx.try_iter().count(), 4);

        let (url, rx) = serve(vec![(400, String::new()), (200, String::new())]);
        assert_eq!(hook(url).send(&doc).unwrap_err().to_string(), "http_400");
        assert_eq!(rx.try_iter().count(), 1);
    }
}
