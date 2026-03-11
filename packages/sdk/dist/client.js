let _config = {
    url: process.env.MEMKIT_URL ?? "https://api.memkit.io",
};
export function getConfig() {
    return { ..._config };
}
export function setConfig(config) {
    if (config.url !== undefined)
        _config.url = config.url;
}
async function fetchWithTimeout(url, options, timeoutMs = 120000) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs);
    try {
        const res = await fetch(url, {
            ...options,
            signal: controller.signal,
        });
        return res;
    }
    finally {
        clearTimeout(timeout);
    }
}
export async function clientGet(path) {
    const { url } = getConfig();
    const res = await fetchWithTimeout(`${url}${path}`, { method: "GET" });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(`memkit: ${res.status} ${text}`);
    }
    return res.json();
}
export async function clientPost(path, body) {
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
