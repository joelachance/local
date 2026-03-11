import type { MemkitConfig } from "./types.js";

let _config: MemkitConfig = {
  url: process.env.MEMKIT_URL ?? "https://api.memkit.io",
};

export function getConfig(): MemkitConfig {
  return { ..._config };
}

export function setConfig(config: Partial<MemkitConfig>): void {
  if (config.url !== undefined) _config.url = config.url;
}

async function fetchWithTimeout(
  url: string,
  options: RequestInit,
  timeoutMs = 120_000
): Promise<Response> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, {
      ...options,
      signal: controller.signal,
    });
    return res;
  } finally {
    clearTimeout(timeout);
  }
}

export async function clientGet(path: string): Promise<unknown> {
  const { url } = getConfig();
  const res = await fetchWithTimeout(`${url}${path}`, { method: "GET" });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`memkit: ${res.status} ${text}`);
  }
  return res.json();
}

export async function clientPost(
  path: string,
  body: unknown
): Promise<unknown> {
  const { url } = getConfig();
  const res = await fetchWithTimeout(`${url}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`memkit: ${res.status} ${text}`);
  }
  return res.json();
}
