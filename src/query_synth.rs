use anyhow::{Context, Result};

use crate::ontology::LlmConfig;
use crate::ontology_llama::generate_completion;
use crate::types::QueryResponse;

const MAX_CONTEXT_CHARS: usize = 8000;
const MAX_CHUNKS: usize = 8;
const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryProvider {
    Llama,
    OpenAI(String),
}

impl QueryProvider {
    pub fn label(&self) -> String {
        match self {
            QueryProvider::Llama => "Internal: Llama".to_string(),
            QueryProvider::OpenAI(model) => format!("OpenAI: {}", model),
        }
    }
}

pub fn synthesize_answer(query: &str, response: &QueryResponse) -> Result<(String, QueryProvider)> {
    if response.results.is_empty() {
        return Ok((
            "No relevant context found in the memory pack.".to_string(),
            QueryProvider::Llama,
        ));
    }

    let prompt_inner = build_prompt_inner(query, response);
    let config = LlmConfig::from_env();

    // #region agent log
    let env_result = std::env::var("OPENAI_API_KEY");
    let (has_key, key_len, no_key_reason) = match &env_result {
        Ok(k) => (true, k.len(), Option::<String>::None),
        Err(e) => (false, 0, Some(e.to_string())),
    };
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let line = serde_json::json!({"sessionId":"ef491a","timestamp":ts,"location":"query_synth.rs:synthesize_answer","message":"OPENAI_API_KEY check","data":{"has_key":has_key,"key_len":key_len,"no_key_reason":no_key_reason},"hypothesisId":"A"});
    let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/joe/git/local/.cursor/debug-ef491a.log").and_then(|mut f| std::io::Write::write_all(&mut f, (serde_json::to_string(&line).unwrap_or_default() + "\n").as_bytes()));
    // #endregion

    // Prefer OpenAI when OPENAI_API_KEY is set; fall back to embedded model on failure or when no key.
    if let Ok(api_key) = env_result {
        if !api_key.trim().is_empty() {
            let model = std::env::var("MEMKIT_OPENAI_MODEL")
                .unwrap_or_else(|_| DEFAULT_OPENAI_MODEL.to_string());
            // #region agent log
            let ts2 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            let line2 = serde_json::json!({"sessionId":"ef491a","timestamp":ts2,"location":"query_synth.rs:synthesize_answer","message":"attempting OpenAI","data":{"model":&model},"hypothesisId":"C"});
            let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/joe/git/local/.cursor/debug-ef491a.log").and_then(|mut f| std::io::Write::write_all(&mut f, (serde_json::to_string(&line2).unwrap_or_default() + "\n").as_bytes()));
            // #endregion
            match openai_completion(&prompt_inner, config.max_tokens, &model, &api_key) {
                Ok(out) => {
                    let answer = if std::env::var("MEMKIT_QUERY_RAW_ANSWER").as_deref() == Ok("1") {
                        out.trim().to_string()
                    } else {
                        truncate_answer(&out)
                    };
                    return Ok((answer, QueryProvider::OpenAI(model)));
                }
                Err(e) => {
                    // #region agent log
                    let ts3 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
                    let line3 = serde_json::json!({"sessionId":"ef491a","timestamp":ts3,"location":"query_synth.rs:synthesize_answer","message":"OpenAI failed, falling back","data":{"err":e.to_string()},"hypothesisId":"C"});
                    let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/joe/git/local/.cursor/debug-ef491a.log").and_then(|mut f| std::io::Write::write_all(&mut f, (serde_json::to_string(&line3).unwrap_or_default() + "\n").as_bytes()));
                    // #endregion
                }
            }
        } else {
            // #region agent log
            let ts4 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            let line4 = serde_json::json!({"sessionId":"ef491a","timestamp":ts4,"location":"query_synth.rs:synthesize_answer","message":"key empty after trim","data":{},"hypothesisId":"B"});
            let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/joe/git/local/.cursor/debug-ef491a.log").and_then(|mut f| std::io::Write::write_all(&mut f, (serde_json::to_string(&line4).unwrap_or_default() + "\n").as_bytes()));
            // #endregion
        }
    }

    // #region agent log
    let ts5 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let line5 = serde_json::json!({"sessionId":"ef491a","timestamp":ts5,"location":"query_synth.rs:synthesize_answer","message":"using Llama path","data":{},"hypothesisId":"D"});
    let _ = std::fs::OpenOptions::new().create(true).append(true).open("/Users/joe/git/local/.cursor/debug-ef491a.log").and_then(|mut f| std::io::Write::write_all(&mut f, (serde_json::to_string(&line5).unwrap_or_default() + "\n").as_bytes()));
    // #endregion

    if !std::path::Path::new(&config.model).exists() {
        anyhow::bail!(
            "Model file not found: {}. Set MEMKIT_LLM_MODEL to a GGUF path, or build with `cargo build --features llama-embedded` for in-process inference.",
            config.model
        );
    }

    let prompt = build_prompt_llama(&prompt_inner);
    let out = match generate_completion(&prompt, &config, None) {
        Ok(o) => o,
        Err(e) => {
            return Err(e.context(format!("Llama failed (model: {})", config.model)));
        }
    };

    // Set MEMKIT_QUERY_RAW_ANSWER=1 to see the unmodified model output (no cut_at_next_turn, strip_template_tokens, or first-line normalization).
    let answer = if std::env::var("MEMKIT_QUERY_RAW_ANSWER").as_deref() == Ok("1") {
        out.trim().to_string()
    } else {
        truncate_answer(&out)
    };
    Ok((answer, QueryProvider::Llama))
}

fn build_prompt_inner(query: &str, response: &QueryResponse) -> String {
    let mut context = String::with_capacity(MAX_CONTEXT_CHARS + 512);
    for hit in response.results.iter().take(MAX_CHUNKS) {
        let block = format!(
            "{}\n(source: {})\n---\n",
            hit.content.trim(),
            hit.file_path
        );
        if context.len() + block.len() > MAX_CONTEXT_CHARS {
            break;
        }
        context.push_str(&block);
    }
    format!(
        "Using only the context below, answer the question in 1-2 sentences. If the context contains relevant numbers, amounts, or facts, state them. Only say you cannot determine the answer if the context truly does not contain the information.\n\nQuestion: {query}\n\nContext:\n---\n{context}\n\nReply:"
    )
}

fn build_prompt_llama(prompt_inner: &str) -> String {
    format!("<|user|>\n{prompt_inner}\n<|assistant|>\n")
}

fn openai_completion(
    user_message: &str,
    max_tokens: usize,
    model: &str,
    api_key: &str,
) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .build()
        .context("build reqwest client for OpenAI")?;
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": user_message}],
        "max_tokens": max_tokens
    });
    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .context("OpenAI API request failed")?;
    let status = res.status();
    let text = res.text().context("read OpenAI response body")?;
    if !status.is_success() {
        anyhow::bail!("OpenAI API error ({}): {}", status, text);
    }
    let json: serde_json::Value =
        serde_json::from_str(&text).context("parse OpenAI JSON response")?;
    let content = json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("OpenAI response missing choices[0].message.content"))?;
    Ok(content.to_string())
}

/// Strip chat-template tokens that sometimes appear in model output (e.g. |<|user|>|, <|//3//>|).
fn strip_template_tokens(s: &str) -> String {
    let mut out = s.to_string();
    // Remove |<|...|>| style (e.g. |<|user|>|, |<|//3//>|)
    while let Some(start) = out.find("|<|") {
        let rest = &out[start..];
        let end = rest.find("|>|").map(|i| start + i + 3).or_else(|| rest.find(">|").map(|i| start + i + 2));
        if let Some(e) = end {
            out.replace_range(start..e, " ");
        } else {
            break;
        }
    }
    // Remove <|...|> or <|...>|
    while let Some(start) = out.find("<|") {
        let rest = &out[start..];
        let end = rest.find("|>").map(|i| start + i + 2).or_else(|| rest.find(">|").map(|i| start + i + 2));
        if let Some(e) = end {
            out.replace_range(start..e, " ");
        } else {
            break;
        }
    }
    out = out.replace("|_|", " ");
    out.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}

/// Cut at first "next turn" marker so we don't return model continuation (e.g. "|Human: ..." / "|ASSISTANT: ...").
fn cut_at_next_turn(s: &str) -> &str {
    const MARKERS: &[&str] = &[
        "|Human:",
        "|human:",
        "|ASSISTANT:",
        "|Assistant:",
        "|assistant:",
        "Human:",
        "ASSISTANT:",
        "<|user|>",
        "<|assistant|>",
    ];
    let mut cut = s.len();
    for m in MARKERS {
        if let Some(i) = s.find(m) {
            cut = cut.min(i);
        }
    }
    s[..cut].trim_end()
}

/// Clean up model output (strip template tokens, cut at next-turn markers, optional first-line normalization). No length limit.
fn truncate_answer(s: &str) -> String {
    let after_turn = cut_at_next_turn(s);
    let mut trimmed = strip_template_tokens(after_turn).trim().to_string();
    if trimmed.is_empty() {
        trimmed = after_turn.trim().to_string();
    }
    // If model returned a single-line numbered list like "1. answer", use just the answer part
    if let Some(first) = trimmed.lines().next() {
        let first = first.trim();
        if let Some(rest) = first.strip_prefix(|c: char| c.is_ascii_digit()) {
            let rest = rest.trim_start_matches(". \"").trim_start_matches(". ");
            let rest = rest.strip_suffix('"').unwrap_or(rest).trim();
            if !rest.is_empty() {
                trimmed = rest.to_string();
            }
        }
    }
    trimmed
}
