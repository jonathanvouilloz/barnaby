//! Mistral Voxtral cloud transcription, 600 s blocks per request.

use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;
use ureq::unversioned::multipart::{Form, Part};

use crate::audio::{blocks, wav_bytes};
use crate::model::SAMPLE_RATE;
use crate::net::NetPolicy;
use crate::provider::{Coded, RawSegment, Transcriber, Transcript};

/// Provider name, also stored as `meeting.model`.
pub const MISTRAL_MODEL: &str = "mistral-voxtral";

const URL: &str = "https://api.mistral.ai/v1/audio/transcriptions";
const BLOCK_S: f64 = 600.0;
const MAX_BIAS_TERMS: usize = 100;

pub struct MistralTranscriber {
    pub api_key: String,
    /// `Settings.vocabulary`, sent as `context_bias`.
    pub vocabulary: Vec<String>,
    pub policy: NetPolicy,
}

impl Transcriber for MistralTranscriber {
    fn name(&self) -> &str {
        MISTRAL_MODEL
    }

    // `language` is ignored: Mistral refuses it together with `timestamp_granularities`.
    fn transcribe(&self, samples: &[f32], _language: Option<&str>) -> Result<Transcript> {
        self.policy.check(URL)?;
        let terms = bias_terms(&self.vocabulary);
        let agent = crate::net::agent(Duration::from_secs(600));
        let mut out = Transcript {
            language: String::new(),
            segments: Vec::new(),
        };
        for (start, end) in blocks(samples, BLOCK_S) {
            // Mistral may reject a near-empty file (400), which would fail the whole meeting.
            if end - start < SAMPLE_RATE as usize {
                continue;
            }
            let wav = wav_bytes(&samples[start..end])?;
            let mut form = Form::new()
                .text("model", "voxtral-mini-latest")
                .text("timestamp_granularities", "segment");
            for t in &terms {
                form = form.text("context_bias", t);
            }
            let form = form.part(
                "file",
                Part::bytes(&wav)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")?,
            );
            let body = agent
                .post(URL)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send(form)
                .and_then(|mut r| r.body_mut().read_to_string())
                .map_err(|e| match e {
                    ureq::Error::StatusCode(status) => {
                        log::warn!("mistral status {status}");
                        Coded(status_code(status))
                    }
                    e => {
                        log::warn!("mistral transport error: {e}");
                        Coded("mistral_unreachable")
                    }
                })?;
            let rate = f64::from(SAMPLE_RATE);
            let block = parse(&body, start as f64 / rate, (end - start) as f64 / rate)?;
            if out.language.is_empty() {
                out.language = block.language;
            }
            out.segments.extend(block.segments);
        }
        if out.language.is_empty() {
            out.language = "auto".into();
        }
        Ok(out)
    }
}

#[derive(Deserialize)]
struct Response {
    text: String,
    language: Option<String>,
    #[serde(default)]
    segments: Vec<ResponseSegment>,
}

#[derive(Deserialize)]
struct ResponseSegment {
    text: String,
    start: f64,
    end: f64,
}

/// One block's response, times shifted by `offset` s. `duration` (block length in s)
/// bounds the single segment made when Mistral returns text without segments.
fn parse(body: &str, offset: f64, duration: f64) -> Result<Transcript> {
    let r: Response = serde_json::from_str(body).map_err(|_| Coded("mistral_failed"))?;
    let mut segments: Vec<RawSegment> = r
        .segments
        .into_iter()
        .map(|s| RawSegment {
            start: offset + s.start,
            end: offset + s.end,
            text: s.text.trim().to_string(),
            confidence: 1.0,
        })
        .filter(|s| !s.text.is_empty())
        .collect();
    let text = r.text.trim();
    if segments.is_empty() && !text.is_empty() {
        segments.push(RawSegment {
            start: offset,
            end: offset + duration,
            text: text.to_string(),
            confidence: 1.0,
        });
    }
    Ok(Transcript {
        language: r.language.unwrap_or_default(),
        segments,
    })
}

/// Non-empty terms, at most `MAX_BIAS_TERMS`, in order.
fn bias_terms(vocabulary: &[String]) -> Vec<&str> {
    vocabulary
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .take(MAX_BIAS_TERMS)
        .collect()
}

fn status_code(status: u16) -> &'static str {
    match status {
        401 | 403 => "mistral_auth",
        429 => "mistral_quota",
        _ => "mistral_failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProcessingMode;
    use crate::provider::{RawSegment, Transcriber};

    #[test]
    fn parses_segments_and_offsets_them() {
        let body = r#"{"text":"a b","language":"fr","segments":[{"text":" a","start":0.5,"end":1.0,"extra":1},{"text":"b","start":1.2,"end":2.0}],"model":"m","usage":{}}"#;
        let t = parse(body, 600.0, 30.0).unwrap();
        assert_eq!(t.language, "fr");
        assert_eq!(
            t.segments[0],
            RawSegment {
                start: 600.5,
                end: 601.0,
                text: "a".into(),
                confidence: 1.0
            }
        );
        assert_eq!(t.segments.len(), 2);
    }

    #[test]
    fn text_without_segments_becomes_one_segment() {
        for body in [
            r#"{"text":"bonjour","language":null}"#,
            r#"{"text":"bonjour","segments":[]}"#,
        ] {
            let t = parse(body, 600.0, 30.0).unwrap();
            assert_eq!(
                t.segments,
                vec![RawSegment {
                    start: 600.0,
                    end: 630.0,
                    text: "bonjour".into(),
                    confidence: 1.0
                }]
            );
            assert_eq!(t.language, "");
        }
    }

    #[test]
    fn empty_text_gives_no_segment() {
        let t = parse(r#"{"text":"","segments":[]}"#, 0.0, 30.0).unwrap();
        assert!(t.segments.is_empty());
    }

    #[test]
    fn bias_terms_are_capped_and_cleaned() {
        let vocabulary: Vec<String> = (0..150)
            .map(|i| {
                if i % 3 == 0 {
                    " ".into()
                } else {
                    format!("t{i}")
                }
            })
            .collect();
        let terms = bias_terms(&vocabulary);
        assert_eq!(terms.len(), 100);
        assert_eq!(&terms[..3], &["t1", "t2", "t4"]);
        assert!(terms.iter().all(|t| !t.is_empty()));
    }

    #[test]
    fn status_codes_map_to_error_codes() {
        assert_eq!(status_code(401), "mistral_auth");
        assert_eq!(status_code(403), "mistral_auth");
        assert_eq!(status_code(429), "mistral_quota");
        assert_eq!(status_code(500), "mistral_failed");
    }

    #[test]
    fn local_mode_blocks_before_sending() {
        let t = MistralTranscriber {
            api_key: "k".into(),
            vocabulary: vec![],
            policy: NetPolicy {
                mode: ProcessingMode::Local,
            },
        };
        assert_eq!(t.name(), MISTRAL_MODEL);
        let err = t.transcribe(&[0.0; 16_000], None).unwrap_err();
        assert!(crate::net::is_blocked(&err));
    }

    #[test]
    fn short_audio_sends_nothing() {
        let t = MistralTranscriber {
            api_key: "k".into(),
            vocabulary: vec![],
            policy: NetPolicy {
                mode: ProcessingMode::Api,
            },
        };
        // The URL is fixed and remote: an attempted request would fail this test offline.
        for samples in [&[][..], &[0.0; 8_000][..]] {
            let out = t.transcribe(samples, None).unwrap();
            assert_eq!(out.language, "auto");
            assert!(out.segments.is_empty());
        }
    }
}
