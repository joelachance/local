import os

import httpx

_TIMEOUT = 120
_config: dict = {"url": os.environ.get("MEMKIT_URL", "https://api.memkit.io")}


def get_config() -> dict:
    return dict(_config)


def set_config(url: str | None = None) -> None:
    if url is not None:
        _config["url"] = url


def client_get(path: str) -> dict:
    base = _config["url"].rstrip("/")
    url = f"{base}{path}"
    with httpx.Client(timeout=_TIMEOUT) as client:
        resp = client.get(url)
        if not resp.is_success:
            raise RuntimeError(f"memkit: {resp.status_code} {resp.text}")
        return resp.json()


def client_post(path: str, body: dict) -> dict:
    base = _config["url"].rstrip("/")
    url = f"{base}{path}"
    with httpx.Client(timeout=_TIMEOUT) as client:
        resp = client.post(url, json=body)
        if not resp.is_success:
            raise RuntimeError(f"memkit: {resp.status_code} {resp.text}")
        return resp.json()
