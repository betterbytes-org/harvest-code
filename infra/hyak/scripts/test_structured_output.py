#!/usr/bin/env python3
"""Verify vLLM returns JSON matching HARVEST's structured-output schema."""

import json
import os
import sys
import urllib.error
import urllib.request

ENDPOINT = os.environ.get("VLLM_ENDPOINT", "http://127.0.0.1:8000/v1")
MODEL = os.environ.get("VLLM_SERVED_NAME", "deepseek-v4-flash")

SCHEMA = {
    "type": "object",
    "properties": {
        "files": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "contents": {"type": "string"},
                },
                "required": ["path", "contents"],
            },
        }
    },
    "required": ["files"],
}


def post_chat(use_tools: bool) -> dict:
    body = {
        "model": MODEL,
        "messages": [
            {
                "role": "system",
                "content": (
                    "You translate C to Rust. Respond ONLY with valid JSON matching "
                    "the requested schema."
                ),
            },
            {
                "role": "user",
                "content": (
                    'Return JSON for a single Rust file at path "src/main.rs" '
                    'with contents "fn main() { println!(\\"hi\\"); }".'
                ),
            },
        ],
        "max_tokens": 512,
        "temperature": 0.0,
    }
    if use_tools:
        body["tools"] = [
            {
                "type": "function",
                "function": {
                    "name": "file",
                    "description": "Structured file output",
                    "parameters": SCHEMA,
                },
            }
        ]
        body["tool_choice"] = {"type": "function", "function": {"name": "file"}}
    else:
        body["response_format"] = {"type": "json_object"}

    req = urllib.request.Request(
        f"{ENDPOINT}/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.loads(resp.read())


def extract_json(payload: dict) -> dict:
    choice = payload["choices"][0]["message"]
    if choice.get("tool_calls"):
        args = choice["tool_calls"][0]["function"]["arguments"]
        return json.loads(args)
    content = choice.get("content", "").strip()
    if content.startswith("```"):
        lines = content.splitlines()
        content = "\n".join(lines[1:-1] if lines[-1].startswith("```") else lines[1:])
    return json.loads(content)


def main() -> int:
    print(f"Endpoint: {ENDPOINT}  model: {MODEL}")

    # Health check
    try:
        with urllib.request.urlopen(f"{ENDPOINT.replace('/v1', '')}/health", timeout=10):
            print("Health: OK")
    except urllib.error.HTTPError:
        pass  # some vLLM builds only expose /v1/models
    except urllib.error.URLError as e:
        print(f"FAIL: server unreachable: {e}")
        return 1

    for mode, use_tools in [("tool_call", True), ("json_object", False)]:
        print(f"\n--- Testing {mode} ---")
        try:
            payload = post_chat(use_tools=use_tools)
            parsed = extract_json(payload)
            assert "files" in parsed and isinstance(parsed["files"], list)
            assert parsed["files"][0]["path"]
            print(f"PASS ({mode}): {json.dumps(parsed)[:200]}...")
        except Exception as e:
            print(f"FAIL ({mode}): {e}")
            return 1

    print("\nAll structured-output checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
