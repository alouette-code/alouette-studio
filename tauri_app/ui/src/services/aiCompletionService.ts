/**
 * AI Code Completion Service
 * Connects to http://localhost:3001/v1/code/completions (or fallback /v1/chat/completions)
 * Optimised for Groq Free Tier (6,000 TPM limit) with Client-Side Rate Limit Throttling
 */

export interface CodeCompletionRequest {
  prefix: string;
  suffix: string;
  language: string;
  isManualTrigger?: boolean;
}

export interface CodeCompletionResponse {
  completion: string;
}

const SERVER_URL = "http://localhost:3001";
// Optimized context sizes to conserve Groq Free Tier TPM (6000 TPM limit)
const MAX_PREFIX_CHARS = 1600; // ~400 tokens
const MAX_SUFFIX_CHARS = 400;  // ~100 tokens

// Minimum delay between auto-completion requests (1.5 seconds)
const AUTO_TRIGGER_COOLDOWN_MS = 1500;
let lastRequestTime = 0;

/**
 * Extracts prefix token and suffix token window around cursor from Monaco model
 */
export function extractCodeContext(
  model: any,
  position: { lineNumber: number; column: number }
): { prefix: string; suffix: string } {
  if (!model || !position) {
    return { prefix: "", suffix: "" };
  }

  const cursorOffset = model.getOffsetAt(position);
  const fullText = model.getValue();

  // Prefix: up to ~400 tokens before cursor
  const rawPrefix = fullText.slice(0, cursorOffset);
  const prefix = rawPrefix.length > MAX_PREFIX_CHARS
    ? rawPrefix.slice(-MAX_PREFIX_CHARS)
    : rawPrefix;

  // Suffix: up to ~100 tokens after cursor
  const rawSuffix = fullText.slice(cursorOffset);
  const suffix = rawSuffix.length > MAX_SUFFIX_CHARS
    ? rawSuffix.slice(0, MAX_SUFFIX_CHARS)
    : rawSuffix;

  return { prefix, suffix };
}

let activeAbortController: AbortController | null = null;

/**
 * Fetch AI Code Completion from localhost:3001 with Rate Limit Throttling
 */
export async function fetchAiCodeCompletion(
  req: CodeCompletionRequest
): Promise<string> {
  const now = Date.now();

  // Rate Limiting for Auto Trigger to avoid 429 Rate Limit (6,000 TPM) on Groq Free Tier
  if (!req.isManualTrigger) {
    if (now - lastRequestTime < AUTO_TRIGGER_COOLDOWN_MS) {
      return "";
    }
  }

  // Cancel previous pending request if any
  if (activeAbortController) {
    activeAbortController.abort();
  }

  activeAbortController = new AbortController();
  const signal = activeAbortController.signal;
  lastRequestTime = now;

  try {
    let response = await fetch(`${SERVER_URL}/v1/code/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        prefix: req.prefix,
        suffix: req.suffix,
        language: req.language,
        max_tokens: 96,
      }),
      signal,
    });

    if (response.status === 404) {
      // Fallback to /v1/chat/completions (OpenAI Compatible API)
      const systemPrompt = `You are an expert AI code completion engine.
Your task is to generate ONLY the exact code completion snippet to be inserted directly at the cursor position between [PREFIX] and [SUFFIX].
CRITICAL RULES:
1. Output ONLY the raw code snippet to complete the code logic.
2. Do NOT wrap in markdown code blocks (\`\`\`).
3. Do NOT include any explanations, markdown, or comments.
4. If prefix ends with a partial word or line, continue directly from where prefix left off without duplicating characters.`;

      const userPrompt = `Language: ${req.language}

[PREFIX]
${req.prefix}

[SUFFIX]
${req.suffix}

Completion code at cursor:`;

      response = await fetch(`${SERVER_URL}/v1/chat/completions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          messages: [
            { role: "system", content: systemPrompt },
            { role: "user", content: userPrompt },
          ],
          max_tokens: 96,
          temperature: 0.1,
        }),
        signal,
      });

      if (!response.ok) return "";
      const chatData = await response.json();
      let text = chatData.choices?.[0]?.message?.content || "";
      text = text.replace(/^```[a-zA-Z]*\n?/, "").replace(/\n?```$/, "");
      return text;
    }

    if (!response.ok) {
      console.warn(`[AiCompletion] Server returned status ${response.status}`);
      return "";
    }

    const data = await response.json();
    let text = data.completion || "";
    text = text.replace(/^```[a-zA-Z]*\n?/, "").replace(/\n?```$/, "");
    return text;
  } catch (err: any) {
    if (err.name === "AbortError") {
      // Normal cancellation when user keeps typing
      return "";
    }
    console.warn("[AiCompletion] Failed to fetch completion from localhost:3001:", err);
    return "";
  } finally {
    activeAbortController = null;
  }
}
