//! Spelling fixes for user vocabulary (brand names, jargon) in transcribed text.
//!
//! A run of words whose accent- and case-free form is within a small edit
//! distance of a term is replaced by the term's exact spelling. Runs of the
//! term's word count ±1 are tried, so "open router" can become "OpenRouter".

/// Words never replaced on their own, whatever the vocabulary.
const COMMON: &[&str] = &[
    "a", "au", "aux", "avec", "car", "ce", "ces", "cet", "cette", "dans", "de", "des", "donc",
    "du", "elle", "elles", "en", "est", "et", "il", "ils", "je", "la", "le", "les", "leur", "mais",
    "me", "mon", "ne", "nous", "on", "ou", "par", "pas", "pour", "que", "qui", "sa", "se", "son",
    "sont", "sur", "ta", "te", "tes", "ton", "tous", "tout", "tu", "un", "une", "vous", "an",
    "and", "are", "for", "in", "is", "it", "of", "or", "the", "this", "to", "was", "with", "you",
];

struct Token {
    /// Byte range of the word without its surrounding punctuation.
    start: usize,
    end: usize,
    /// True when the word touches whitespace on that side (no punctuation in between).
    bare_left: bool,
    bare_right: bool,
    norm: Vec<char>,
}

struct Term<'a> {
    text: &'a str,
    words: usize,
    norm: Vec<char>,
}

pub fn correct(text: &str, vocabulary: &[String]) -> String {
    let mut terms: Vec<Term> = vocabulary
        .iter()
        .map(|t| Term {
            text: t.trim(),
            words: t.split_whitespace().count(),
            norm: normalize(t),
        })
        .filter(|t| !t.norm.is_empty())
        .collect();
    if terms.is_empty() {
        return text.to_string();
    }
    terms.sort_by_key(|t| std::cmp::Reverse(t.norm.len()));
    let tokens = tokenize(text);

    // (start token, end token exclusive, replacement)
    let mut taken: Vec<(usize, usize, &str)> = Vec::new();
    let overlaps = |taken: &[(usize, usize, &str)], a: usize, b: usize| {
        taken.iter().any(|&(s, e, _)| a < e && s < b)
    };
    let mut joined = Vec::new();
    let mut rows = Default::default();
    for term in &terms {
        let max = allowed_edits(term.norm.len());
        // (distance, word-count mismatch, start, end)
        let mut found: Vec<(usize, usize, usize, usize)> = Vec::new();
        for start in 0..tokens.len() {
            if tokens[start].norm.first() != term.norm.first() {
                continue;
            }
            for words in term.words.saturating_sub(1).max(1)..=term.words + 1 {
                let end = start + words;
                if end > tokens.len() {
                    break;
                }
                let span = &tokens[start..end];
                let len: usize = span.iter().map(|t| t.norm.len()).sum();
                if len.abs_diff(term.norm.len()) > max
                    || span[..words - 1].iter().any(|t| !t.bare_right)
                    || span[1..].iter().any(|t| !t.bare_left)
                    || span.iter().any(|t| t.norm.is_empty())
                {
                    continue;
                }
                joined.clear();
                span.iter().for_each(|t| joined.extend_from_slice(&t.norm));
                if let Some(d) = distance(&joined, &term.norm, max, &mut rows) {
                    if words == 1 && is_common(&joined) {
                        continue;
                    }
                    found.push((d, words.abs_diff(term.words), start, end));
                }
            }
        }
        found.sort_unstable();
        for (_, _, start, end) in found {
            if !overlaps(&taken, start, end) {
                // An exact span is taken too, so a looser overlapping one cannot win.
                taken.push((start, end, term.text));
            }
        }
    }

    taken.sort_unstable_by_key(|&(s, _, _)| s);
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for (s, e, term) in taken {
        let (a, b) = (tokens[s].start, tokens[e - 1].end);
        out.push_str(&text[pos..a]);
        out.push_str(term);
        pos = b;
    }
    out.push_str(&text[pos..]);
    out
}

/// Edits tolerated for a normalized term of `len` characters.
fn allowed_edits(len: usize) -> usize {
    match len {
        0..=3 => 0,
        4..=9 => 1,
        _ => 2,
    }
}

fn is_common(norm: &[char]) -> bool {
    let w: String = norm.iter().collect();
    COMMON.contains(&w.as_str())
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut offset = 0;
    for raw in text.split_inclusive(char::is_whitespace) {
        let word = raw.trim_end_matches(char::is_whitespace);
        let base = offset;
        offset += raw.len();
        if word.is_empty() {
            continue;
        }
        let mut start = word
            .find(|c: char| c.is_alphanumeric())
            .unwrap_or(word.len());
        let end = word
            .rfind(|c: char| c.is_alphanumeric())
            .map_or(start, |i| {
                i + word[i..].chars().next().map_or(1, char::len_utf8)
            });
        // French elision: "l'OpenRouter", "d'Barnaby", "qu'Tauri" -> the word after the apostrophe.
        if let Some(i) = word[start..end].find(['\'', '’']) {
            if (1..=2).contains(&word[start..start + i].chars().count()) {
                start += i + word[start + i..].chars().next().map_or(1, char::len_utf8);
            }
        }
        let core = &word[start..end.max(start)];
        tokens.push(Token {
            start: base + start,
            end: base + end.max(start),
            bare_left: start == 0,
            bare_right: end == word.len(),
            norm: normalize(core),
        });
    }
    tokens
}

/// Lowercase, without accents, letters and digits only.
fn normalize(s: &str) -> Vec<char> {
    s.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| {
            let base = match c {
                'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
                'ç' => 'c',
                'è' | 'é' | 'ê' | 'ë' => 'e',
                'ì' | 'í' | 'î' | 'ï' => 'i',
                'ñ' => 'n',
                'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
                'ù' | 'ú' | 'û' | 'ü' => 'u',
                'ý' | 'ÿ' => 'y',
                c => c,
            };
            base.is_alphanumeric().then_some(base)
        })
        .collect()
}

/// Optimal string alignment distance (Levenshtein + adjacent transpositions),
/// or None when it exceeds `max`. `rows` is scratch space reused across calls.
fn distance(a: &[char], b: &[char], max: usize, rows: &mut [Vec<usize>; 3]) -> Option<usize> {
    let n = b.len();
    let [prev2, prev, cur] = rows;
    for r in [&mut *prev2, &mut *prev, &mut *cur] {
        r.clear();
        r.resize(n + 1, 0);
    }
    prev.iter_mut().enumerate().for_each(|(j, v)| *v = j);
    for i in 1..=a.len() {
        cur[0] = i;
        let mut row_min = i;
        for j in 1..=n {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(prev2[j - 2] + 1);
            }
            cur[j] = v;
            row_min = row_min.min(v);
        }
        if row_min > max {
            return None;
        }
        std::mem::swap(prev2, prev);
        std::mem::swap(prev, cur);
    }
    Some(prev[n]).filter(|d| *d <= max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(text: &str, terms: &[&str]) -> String {
        let v: Vec<String> = terms.iter().map(|t| t.to_string()).collect();
        correct(text, &v)
    }

    const VOCAB: &[&str] = &["Barber Concept", "Barnaby", "Tauri", "OpenRouter"];

    #[test]
    fn fixes_misheard_terms() {
        for (input, want) in [
            (
                "on bosse chez barbor concept",
                "on bosse chez Barber Concept",
            ),
            (
                "on bosse chez barber concepts.",
                "on bosse chez Barber Concept.",
            ),
            ("j'utilise open router", "j'utilise OpenRouter"),
            (
                "j'utilise Open-Router, pas Ollama",
                "j'utilise OpenRouter, pas Ollama",
            ),
            ("l'openrouter marche", "l'OpenRouter marche"),
            ("Demande à Barnabé !", "Demande à Barnaby !"),
            ("une app taury", "une app Tauri"),
            ("BARNABY, tu es là ?", "Barnaby, tu es là ?"),
            (
                "chez barber concept (genève)",
                "chez Barber Concept (genève)",
            ),
        ] {
            assert_eq!(fix(input, VOCAB), want, "{input}");
        }
    }

    #[test]
    fn leaves_ordinary_text_alone() {
        for input in [
            "le concept est bon",
            "un bar concept",
            "tori est arrivé", // 2 edits from "tauri": too far for 5 letters
            "on ouvre la route",
            "open, router",
            "Barber Concept et Barnaby",
            "",
        ] {
            assert_eq!(fix(input, VOCAB), input, "{input}");
        }
        assert_eq!(fix("la", &["La"]), "la");
        assert_eq!(fix("barbor concept", &[]), "barbor concept");
    }

    #[test]
    fn longest_term_wins_without_overlap() {
        assert_eq!(
            fix("barbor concept", &["Concept", "Barber Concept"]),
            "Barber Concept"
        );
        // Does not swallow a neighbouring word.
        assert_eq!(
            fix("barber concept le matin", &["Barber Concept"]),
            "Barber Concept le matin"
        );
    }

    #[test]
    fn short_terms_need_exact_match() {
        assert_eq!(fix("une api rest", &["API"]), "une API rest");
        assert_eq!(fix("une apo rest", &["API"]), "une apo rest");
        assert_eq!(fix("on part sur jirra", &["Jira"]), "on part sur Jira");
    }

    #[test]
    fn is_idempotent() {
        let vocab: Vec<String> = (0..46)
            .map(|i| format!("Terme{i} Special"))
            .chain(VOCAB.iter().map(|s| s.to_string()))
            .collect();
        let text = "on bosse chez barbor concept avec open router et taury ".repeat(25);
        let once = correct(&text, &vocab);
        // ~200 words x 50 terms: measured ~0.6 ms in release on a loaded 2-CPU VPS.
        assert_eq!(correct(&once, &vocab), once);
        assert!(once.contains("Barber Concept avec OpenRouter et Tauri"));
    }
}
