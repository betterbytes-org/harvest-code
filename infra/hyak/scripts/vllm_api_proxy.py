#!/usr/bin/env python3
"""Rate-limited reverse proxy in front of vLLM (stdlib only)."""

import json
import os
import sys
import threading
import time
from http.client import HTTPConnection, HTTPException
from http.server import BaseHTTPRequestHandler, HTTPServer
from socketserver import ThreadingMixIn
from typing import ClassVar, Dict, Optional, Set


class ThreadingHTTPServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True
    allow_reuse_address = True


def _env_bool(name: str, default: bool) -> bool:
    raw = os.environ.get(name)
    if raw is None:
        return default
    return raw.lower() in {"1", "true", "yes", "on"}


def _env_float(name: str, default: float) -> float:
    raw = os.environ.get(name)
    if raw is None:
        return default
    return float(raw)


class TokenBucket:
    """Token bucket rate limiter."""

    def __init__(self, rate_per_minute: float, burst: float) -> None:
        self.rate = rate_per_minute / 60.0
        self.burst = burst
        self.tokens = burst
        self.last = time.monotonic()
        self.lock = threading.Lock()

    def allow(self) -> bool:
        with self.lock:
            now = time.monotonic()
            elapsed = now - self.last
            self.last = now
            self.tokens = min(self.burst, self.tokens + elapsed * self.rate)
            if self.tokens >= 1.0:
                self.tokens -= 1.0
                return True
            return False


class RateLimiter:
    """Per-client token buckets keyed by API key or client IP."""

    def __init__(self, rpm: float, burst: float) -> None:
        self.rpm = rpm
        self.burst = burst
        self.buckets: Dict[str, TokenBucket] = {}
        self.lock = threading.Lock()

    def allow(self, key: str) -> bool:
        with self.lock:
            bucket = self.buckets.get(key)
            if bucket is None:
                bucket = TokenBucket(self.rpm, self.burst)
                self.buckets[key] = bucket
        return bucket.allow()


class VllmProxyHandler(BaseHTTPRequestHandler):
    backend_host: ClassVar[str] = "127.0.0.1"
    backend_port: ClassVar[int] = 8001
    api_keys: ClassVar[Set[str]] = set()
    rate_limiter: ClassVar[Optional[RateLimiter]] = None
    rate_limit_enabled: ClassVar[bool] = True

    def log_message(self, fmt: str, *args: object) -> None:
        sys.stderr.write(f"[vllm-proxy] {self.address_string()} - {fmt % args}\n")

    def _extract_bearer(self) -> Optional[str]:
        auth = self.headers.get("Authorization", "")
        scheme, _, token = auth.partition(" ")
        if scheme.lower() != "bearer" or not token:
            return None
        return token

    def _client_key(self, token: Optional[str]) -> str:
        if token:
            return f"key:{token[:16]}"
        return f"ip:{self.client_address[0]}"

    def _json_error(self, status: int, message: str) -> None:
        body = json.dumps({"error": message}).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _forward(self) -> None:
        token = self._extract_bearer()
        if not token or token not in self.api_keys:
            self._json_error(401, "Unauthorized")
            return

        if self.rate_limit_enabled and self.rate_limiter is not None:
            if not self.rate_limiter.allow(self._client_key(token)):
                self.send_response(429)
                self.send_header("Content-Type", "application/json")
                self.send_header("Retry-After", "60")
                body = json.dumps({"error": "Rate limit exceeded"}).encode()
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
                return

        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length) if length else b""

        headers = {
            k: v
            for k, v in self.headers.items()
            if k.lower() not in {"host", "content-length", "connection"}
        }

        try:
            conn = HTTPConnection(self.backend_host, self.backend_port, timeout=600)
            conn.request(self.command, self.path, body=body, headers=headers)
            resp = conn.getresponse()
        except (HTTPException, OSError) as exc:
            self._json_error(502, f"Backend unavailable: {exc}")
            return

        self.send_response(resp.status)
        for key, value in resp.getheaders():
            if key.lower() not in {"transfer-encoding", "connection"}:
                self.send_header(key, value)
        self.end_headers()

        while True:
            chunk = resp.read(8192)
            if not chunk:
                break
            self.wfile.write(chunk)
        conn.close()

    def do_GET(self) -> None:
        self._forward()

    def do_POST(self) -> None:
        self._forward()

    def do_PUT(self) -> None:
        self._forward()

    def do_DELETE(self) -> None:
        self._forward()

    def do_OPTIONS(self) -> None:
        self.send_response(204)
        self.end_headers()


def main() -> int:
    listen_host = os.environ.get("VLLM_PROXY_HOST", "0.0.0.0")
    listen_port = int(os.environ.get("VLLM_PORT", "8000"))
    VllmProxyHandler.backend_port = int(os.environ.get("VLLM_INTERNAL_PORT", "8001"))

    api_key = os.environ.get("VLLM_API_KEY", "")
    if not api_key:
        print("FAIL: VLLM_API_KEY must be set", file=sys.stderr)
        return 1
    VllmProxyHandler.api_keys = {api_key}

    VllmProxyHandler.rate_limit_enabled = _env_bool("RATE_LIMIT_ENABLED", True)
    if VllmProxyHandler.rate_limit_enabled:
        rpm = _env_float("RATE_LIMIT_REQUESTS_PER_MINUTE", 60.0)
        burst = _env_float("RATE_LIMIT_BURST", 10.0)
        VllmProxyHandler.rate_limiter = RateLimiter(rpm, burst)
        print(
            f"Rate limit: {rpm} req/min, burst {burst}",
            file=sys.stderr,
        )
    else:
        print("Rate limit: disabled", file=sys.stderr)

    server = ThreadingHTTPServer((listen_host, listen_port), VllmProxyHandler)
    print(
        f"Proxy listening on {listen_host}:{listen_port} -> "
        f"127.0.0.1:{VllmProxyHandler.backend_port}",
        file=sys.stderr,
    )
    server.serve_forever()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
