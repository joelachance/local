# What we do to the LLM response (query synthesis)

Exact path from model output → what the user sees.

---

## 1. Prompt we send (Llama path)

```
<|user|>
Using only the context below, answer the question in 1-2 sentences. ...
Question: {query}
Context: ---
{context}

Reply:
<|assistant|>
```

So the model sees a single user message and `Reply:` then `<|assistant|>\n`, and we start sampling. The model generates tokens until EOS, timeout, or max_tokens.

---

## 2. During generation (ontology_llama.rs, embedded path only)

- **EOS:** If the model emits an end-of-sequence token, we stop.
- **Stop strings:** If the accumulated output contains any of  
  `|Human:`, `|ASSISTANT:`, `<|user|>`, `<|assistant|>`  
  we stop the loop immediately (we don’t generate more tokens after that).
- **Timeout / max_tokens:** We also stop on timeout or after max_tokens.

So the **raw** string we return from `run_completion` may already have been cut at one of those stop strings. Everything after that is never generated.

---

## 3. After generation: `truncate_answer()` (query_synth.rs)

Called on the raw string for both Llama and OpenAI. Steps in order:

### 3a. `cut_at_next_turn(s)`

- Take the string and find the **first** occurrence of any of:
  - `|Human:`, `|human:`, `|ASSISTANT:`, `|Assistant:`, `|assistant:`
  - `Human:`, `ASSISTANT:`
  - `<|user|>`, `<|assistant|>`
- Keep only the part **before** that, and trim trailing whitespace.
- So if the model output is:  
  `Good answer here. |Human: Thank you...`  
  we keep only:  
  `Good answer here.`

### 3b. `strip_template_tokens(after_turn)`

- Replace any `|<|...|>|` (e.g. `|<|user|>|`, `|<|//3//>|`) with a space.
- Replace any `<|...|>` or `<|...>|` with a space.
- Replace `|_|` with a space.
- Then: **collapse all whitespace** (split on whitespace, rejoin with single spaces) and trim.

So we remove chat/template tokens and normalize spaces. We do **not** remove or change “normal” text.

### 3c. First-line “1. answer” handling

- If the result has at least one line:
  - Take the first line.
  - If it looks like a single numbered line, e.g. `1. "actual answer"` or `2. answer text`, we strip the leading number and optional quotes and use that as the new result.
- So we only touch the **first line** when it’s clearly a numbered-list style; we don’t truncate or drop the rest of the answer.

### 3d. Return

- We return that final string. There is **no** character or token limit applied here; we only cut at next-turn markers, strip template tokens, and optionally normalize the first line.

---

## 4. Summary table

| Step | Where | What we do |
|------|--------|------------|
| Stop on EOS | ontology_llama | Stop generation when model emits EOS token. |
| Stop on strings | ontology_llama | Stop generation when output contains `\|Human:`, `\|ASSISTANT:`, `<\|user\|>`, `<\|assistant\|>`. |
| cut_at_next_turn | query_synth | Keep only text before first “next turn” marker; trim end. |
| strip_template_tokens | query_synth | Remove `\<\|...\|\>`-style tokens and `\|_\|`; collapse whitespace. |
| First-line “1. answer” | query_synth | If first line is “N. answer”, use “answer” as the result. |

We do **not**:

- Truncate by character count.
- Truncate by token count in post-processing (only in the generation loop via max_tokens).
- Add or rewrite content; we only remove or shorten at markers and strip template tokens.

If you want to see the **unmodified** model output (e.g. to confirm the “|Human:” / “|ASSISTANT:” text is from the model), we can add a `--raw-answer` or env flag that skips `truncate_answer` and prints the raw string.
