//! Big-model oracle leg (Directive 1 §4, Directive 2 §21).
//!
//! A substantially larger model is used as **reference/oracle**, never as part
//! of the deployed system. Transport is an OpenAI-compatible HTTP endpoint
//! (OpenRouter by default) behind the `http` feature, driven by env:
//!
//! ```text
//! PHYSIS_ORACLE_URL    (default https://openrouter.ai/api/v1)
//! PHYSIS_ORACLE_KEY    (fallback: OPENROUTER_API_KEY, then GROQ_API_KEY)
//! PHYSIS_ORACLE_MODEL  (default openai/gpt-4o)
//! ```
//!
//! Offline or keyless ⇒ the leg reports `not_configured`/`failed` and the
//! benchmark carries on measuring everything else. Nothing is ever faked.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleConfig {
    pub url: String,
    pub model: String,
    pub has_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleCase {
    pub query: String,
    pub context_chars: usize,
    pub decisive_docs: Vec<String>,
    pub must_mention: Vec<String>,
    pub physis_answer: String,
    pub oracle_answer: String,
    pub agreement_jaccard: f32,
    pub oracle_cites_evidence: bool,
    pub latency_ms: f64,
}

/// Resolve config from env. `None` ⇒ nothing to call; reported honestly.
pub fn from_env() -> Option<(OracleConfig, String)> {
    let url = std::env::var("PHYSIS_ORACLE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://openrouter.ai/api/v1".into());
    let key = std::env::var("PHYSIS_ORACLE_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("OPENROUTER_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
        .or_else(|| {
            std::env::var("GROQ_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty())
        });
    let model = std::env::var("PHYSIS_ORACLE_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "openai/gpt-4o".into());
    key.map(|k| {
        (
            OracleConfig {
                url,
                model,
                has_key: true,
            },
            k,
        )
    })
}

/// Token Jaccard over lowercase alnum runs ≥3 chars — the linguistic-agreement
/// input. Deterministic, dependency-free, in-repo so tests own it.
pub fn token_jaccard(a: &str, b: &str) -> f32 {
    let toks = |t: &str| -> std::collections::BTreeSet<String> {
        t.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .map(|w| w.to_lowercase())
            .collect()
    };
    let (a, b) = (toks(a), toks(b));
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(&b).count();
    let uni = a.union(&b).count().max(1);
    inter as f32 / uni as f32
}

/// Do all `must_mention` tokens literally occur (case-insensitive)? The
/// decisive-evidence check: did the model recover the facts that decide?
pub fn cites_evidence(answer: &str, must_mention: &[String]) -> bool {
    let low = answer.to_lowercase();
    must_mention.iter().all(|m| low.contains(&m.to_lowercase()))
}

/// One OpenAI-compatible chat call. `max_tokens` is tight: we buy reference
/// behaviour, not essays.
#[cfg(feature = "http")]
pub fn complete(
    cfg: &OracleConfig,
    key: &str,
    system: &str,
    prompt: &str,
    max_tokens: u32,
) -> anyhow::Result<(String, f64)> {
    let started = std::time::Instant::now();
    let body = serde_json::json!({
        "model": cfg.model,
        "temperature": 0,
        "max_tokens": max_tokens,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt},
        ],
    });
    let url = format!("{}/chat/completions", cfg.url.trim_end_matches('/'));
    let resp = ureq::post(&url)
        .set("Authorization", &format!("Bearer {key}"))
        .set("Content-Type", "application/json")
        .set("HTTP-Referer", "https://github.com/ilPez00/physis-core")
        .set("X-Title", "physis-core oracle benchmark")
        .send_string(&serde_json::to_string(&body)?)
        .map_err(|e| anyhow::anyhow!("oracle request: {e}"))?;
    let text = resp
        .into_string()
        .map_err(|e| anyhow::anyhow!("oracle body: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("oracle json: {e}: {text}"))?;
    let content = v
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("oracle shape: {text}"))?
        .to_string();
    Ok((content, started.elapsed().as_secs_f32() as f64 * 1000.0))
}

#[cfg(not(feature = "http"))]
pub fn complete(
    _cfg: &OracleConfig,
    _key: &str,
    _system: &str,
    _prompt: &str,
    _max_tokens: u32,
) -> anyhow::Result<(String, f64)> {
    anyhow::bail!("oracle needs the `http` feature — rebuild with --features http")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jaccard_is_sane() {
        assert!((token_jaccard("the pump failed", "the pump failed") - 1.0).abs() < 1e-6);
        assert!(token_jaccard("invoice approved today", "quantum origami folds") < 0.2);
        assert!(token_jaccard("", "") > 0.99);
    }

    #[test]
    fn evidence_check_requires_every_fact() {
        let m = vec!["2.6".into(), "4.2".into()];
        assert!(cites_evidence("opens at 2.6 bar, counterpart 4.2 bar", &m));
        assert!(!cites_evidence("opens at 2.6 bar", &m));
    }

    #[test]
    fn env_resolution_prefers_explicit_key() {
        std::env::set_var("PHYSIS_ORACLE_KEY", "k-explicit");
        std::env::set_var("PHYSIS_ORACLE_MODEL", "test/big");
        let (cfg, key) = from_env().expect("key set");
        assert_eq!(key, "k-explicit");
        assert_eq!(cfg.model, "test/big");
        std::env::remove_var("PHYSIS_ORACLE_KEY");
        std::env::remove_var("PHYSIS_ORACLE_MODEL");
    }
}
