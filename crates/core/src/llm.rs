//! OpenAI-compatible structuring client (Ollama, OpenRouter, ...).

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::export::speaker_name;
use crate::model::{MeetingDoc, Summary};
use crate::net::NetPolicy;
use crate::provider::Structurer;

pub struct OpenAiStructurer {
    /// e.g. "http://localhost:11434/v1"
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    /// `Settings.vocabulary`, spelled out in the prompt.
    pub vocabulary: Vec<String>,
    pub policy: NetPolicy,
}

const SYSTEM_PROMPT: &str = "You summarize meeting transcripts. Reply with JSON only, matching the schema: \
overview (a short paragraph), decisions, action_items (owner is a participant name or null, due is a date \
or null), open_questions. Use only what the transcript says. Write in the meeting language";

impl Structurer for OpenAiStructurer {
    fn name(&self) -> &str {
        &self.model
    }

    fn structure(&self, doc: &MeetingDoc) -> Result<Summary> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        self.policy.check(&url)?;
        // OpenRouter's optional app attribution; other servers ignore these headers.
        let mut req = crate::net::agent(Duration::from_secs(600))
            .post(&url)
            .header(
                "HTTP-Referer",
                "https://github.com/jonathanvouilloz/barnaby",
            )
            .header("X-Title", "Barnaby");
        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let reply: Value = req
            .send_json(request_body(&self.model, &self.vocabulary, doc))?
            .body_mut()
            .read_json()?;
        let content = reply["choices"][0]["message"]["content"]
            .as_str()
            .context("llm_bad_response")?;
        serde_json::from_str(content).context("llm_bad_json")
    }
}

fn request_body(model: &str, vocabulary: &[String], doc: &MeetingDoc) -> Value {
    let language = doc.meeting.language.as_deref().unwrap_or("auto");
    let mut system = format!("{SYSTEM_PROMPT} (language code: {language}).");
    if !vocabulary.is_empty() {
        system.push_str(&format!(
            " Spell these terms exactly: {}.",
            vocabulary.join(", ")
        ));
    }
    let transcript: Vec<String> = doc
        .segments
        .iter()
        .map(|s| {
            format!(
                "{}: {}",
                speaker_name(doc, s.speaker_id.as_deref()),
                s.text.trim()
            )
        })
        .collect();
    json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": transcript.join("\n") },
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": { "name": "summary", "strict": true, "schema": summary_schema() },
        },
    })
}

fn summary_schema() -> Value {
    let strings = json!({ "type": "array", "items": { "type": "string" } });
    let nullable = json!({ "type": ["string", "null"] });
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["overview", "decisions", "action_items", "open_questions"],
        "properties": {
            "overview": { "type": "string" },
            "decisions": strings,
            "action_items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["owner", "task", "due"],
                    "properties": { "owner": nullable, "task": { "type": "string" }, "due": nullable },
                },
            },
            "open_questions": strings,
        },
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::destination::tests::serve;
    use crate::model::ProcessingMode;

    pub(crate) fn chat_reply(summary: &Summary) -> String {
        json!({ "choices": [{ "message": { "role": "assistant", "content": serde_json::to_string(summary).unwrap() } }] })
            .to_string()
    }

    #[test]
    fn posts_schema_request_and_parses_summary() {
        let doc = crate::export::sample_doc();
        let expected = doc.summary.clone().unwrap();
        let (url, rx) = serve(vec![(200, chat_reply(&expected))]);
        let s = OpenAiStructurer {
            base_url: format!("{url}/v1/"),
            model: "m".into(),
            api_key: Some("k".into()),
            vocabulary: vec!["Barber Concept".into(), "OpenRouter".into()],
            policy: NetPolicy {
                mode: ProcessingMode::Local,
            },
        };
        assert_eq!(s.structure(&doc).unwrap(), expected);
        let (headers, body) = rx.recv().unwrap();
        assert!(headers[0].starts_with("POST /v1/chat/completions "));
        assert!(headers
            .iter()
            .any(|h| h.eq_ignore_ascii_case("authorization: Bearer k")));
        assert!(headers
            .iter()
            .any(|h| h.eq_ignore_ascii_case("x-title: Barnaby")));
        let body: Value = serde_json::from_str(&body).unwrap();
        assert!(body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .ends_with("Spell these terms exactly: Barber Concept, OpenRouter."));
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        assert_eq!(
            body["messages"][1]["content"],
            "Moi: Bonjour\nspeaker_a: Salut"
        );
    }
}
