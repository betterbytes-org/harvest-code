#!/usr/bin/env python3
"""GPU + LLM inference performance benchmark for HARVEST on Hyak."""

from __future__ import annotations

import json
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from typing import Any


@dataclass
class Section:
    title: str
    rows: list[tuple[str, str]] = field(default_factory=list)

    def add(self, key: str, value: str | float | int) -> None:
        if isinstance(value, float):
            self.rows.append((key, f"{value:.4f}"))
        else:
            self.rows.append((key, str(value)))


REPORT: list[Section] = []


def section(title: str) -> Section:
    s = Section(title=title)
    REPORT.append(s)
    return s


def run_cmd(cmd: list[str]) -> str:
    return subprocess.check_output(cmd, text=True, stderr=subprocess.STDOUT)


def benchmark_gpu_compute() -> None:
    sec = section("GPU Compute (PyTorch matmul)")
    try:
        import torch
    except ImportError:
        sec.add("status", "SKIP — torch not installed")
        return

    if not torch.cuda.is_available():
        sec.add("status", "FAIL — CUDA unavailable")
        return

    device_count = torch.cuda.device_count()
    sec.add("cuda_devices", device_count)
    for i in range(device_count):
        props = torch.cuda.get_device_properties(i)
        sec.add(
            f"gpu_{i}",
            f"{props.name}, {props.total_memory / (1024**3):.1f} GiB, "
            f"SMs={props.multi_processor_count}, CC={props.major}.{props.minor}",
        )

    sizes = [2048, 4096, 8192]
    for n in sizes:
        a = torch.randn(n, n, device="cuda", dtype=torch.float16)
        b = torch.randn(n, n, device="cuda", dtype=torch.float16)
        # warmup
        for _ in range(3):
            _ = a @ b
        torch.cuda.synchronize()
        reps = 10
        t0 = time.perf_counter()
        for _ in range(reps):
            c = a @ b
        torch.cuda.synchronize()
        elapsed = time.perf_counter() - t0
        flops = 2 * n**3 * reps
        tflops = flops / elapsed / 1e12
        sec.add(f"matmul_{n}x{n}_fp16_tflops", tflops)
        sec.add(f"matmul_{n}x{n}_fp16_ms_per_iter", (elapsed / reps) * 1000)


def wait_for_ollama(host: str, timeout_s: int = 120) -> bool:
    url = f"http://{host}/api/tags"
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=2) as resp:
                if resp.status == 200:
                    return True
        except (urllib.error.URLError, TimeoutError):
            time.sleep(2)
    return False


def ollama_generate(
    host: str,
    model: str,
    prompt: str,
    num_predict: int,
    stream: bool = False,
) -> dict[str, Any]:
    payload = {
        "model": model,
        "prompt": prompt,
        "stream": stream,
        "options": {"num_predict": num_predict, "temperature": 0.0},
    }
    data = json.dumps(payload).encode()
    req = urllib.request.Request(
        f"http://{host}/api/generate",
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    if not stream:
        with urllib.request.urlopen(req, timeout=600) as resp:
            return json.loads(resp.read())

    t_start = time.perf_counter()
    ttft_s: float | None = None
    chunks: list[str] = []
    with urllib.request.urlopen(req, timeout=600) as resp:
        for raw_line in resp:
            line = raw_line.decode().strip()
            if not line:
                continue
            obj = json.loads(line)
            if obj.get("response"):
                if ttft_s is None:
                    ttft_s = time.perf_counter() - t_start
                chunks.append(obj["response"])
            if obj.get("done"):
                obj["stream_ttft_s"] = ttft_s
                obj["stream_total_s"] = time.perf_counter() - t_start
                return obj
    raise RuntimeError("ollama stream ended without done=true")


def ns_to_s(ns: int | float) -> float:
    return float(ns) / 1e9


def benchmark_ollama(host: str, model: str) -> None:
    sec = section(f"LLM Inference — Ollama ({model})")
    if not wait_for_ollama(host):
        sec.add("status", f"FAIL — Ollama not reachable at {host}")
        return

    with urllib.request.urlopen(f"http://{host}/api/tags", timeout=5) as resp:
        tags = json.loads(resp.read())
    models = [m.get("name", "") for m in tags.get("models", [])]
    sec.add("available_models", ", ".join(models) or "(none)")
    if not any(model.split(":")[0] in m for m in models):
        sec.add("status", f"WARN — {model} not in model list; attempting anyway")

    scenarios = [
        ("short_prompt_64tok", "Say hello in one sentence.", 64),
        ("medium_prompt_256tok", "Explain how a binary search tree works. " * 8, 128),
        (
            "long_prompt_512tok",
            "Translate the following C function to idiomatic Rust with comments:\n"
            + ("int add(int a, int b) { return a + b; }\n" * 40),
            256,
        ),
        ("code_gen_512tok", "Write a complete Rust module implementing a thread-safe LRU cache.", 512),
    ]

    for name, prompt, num_predict in scenarios:
        # Warmup
        ollama_generate(host, model, "warmup", 8, stream=False)

        latencies: list[float] = []
        prompt_rates: list[float] = []
        gen_rates: list[float] = []
        ttfts: list[float] = []

        runs = 3
        for _ in range(runs):
            result = ollama_generate(host, model, prompt, num_predict, stream=True)
            prompt_tokens = result.get("prompt_eval_count", 0)
            gen_tokens = result.get("eval_count", 0)
            prompt_s = ns_to_s(result.get("prompt_eval_duration", 0))
            gen_s = ns_to_s(result.get("eval_duration", 0))
            total_s = result.get("stream_total_s") or (prompt_s + gen_s)
            ttft = result.get("stream_ttft_s")
            if ttft is not None:
                ttfts.append(ttft)
            latencies.append(total_s)
            if prompt_s > 0 and prompt_tokens > 0:
                prompt_rates.append(prompt_tokens / prompt_s)
            if gen_s > 0 and gen_tokens > 0:
                gen_rates.append(gen_tokens / gen_s)

        sec.add(f"{name}_prompt_chars", len(prompt))
        sec.add(f"{name}_num_predict", num_predict)
        sec.add(f"{name}_latency_s_mean", statistics.mean(latencies))
        sec.add(f"{name}_latency_s_stdev", statistics.pstdev(latencies) if len(latencies) > 1 else 0.0)
        if ttfts:
            sec.add(f"{name}_ttft_s_mean", statistics.mean(ttfts))
        if prompt_rates:
            sec.add(f"{name}_prompt_tok_per_s_mean", statistics.mean(prompt_rates))
        if gen_rates:
            sec.add(f"{name}_gen_tok_per_s_mean", statistics.mean(gen_rates))
            sec.add(f"{name}_gen_tok_per_s_max", max(gen_rates))

    # Sustained throughput: back-to-back requests
    sustained = section(f"Sustained Throughput — Ollama ({model})")
    prompt = "List five Rust ownership rules, briefly."
    num_predict = 128
    count = 5
    t0 = time.perf_counter()
    total_gen_tokens = 0
    for _ in range(count):
        r = ollama_generate(host, model, prompt, num_predict, stream=False)
        total_gen_tokens += r.get("eval_count", 0)
    elapsed = time.perf_counter() - t0
    sustained.add("requests", count)
    sustained.add("total_gen_tokens", total_gen_tokens)
    sustained.add("wall_time_s", elapsed)
    sustained.add("aggregate_gen_tok_per_s", total_gen_tokens / elapsed if elapsed else 0)
    sustained.add("requests_per_s", count / elapsed if elapsed else 0)


def print_report(hostname: str) -> None:
    print("=" * 72)
    print("HARVEST GPU PERFORMANCE REPORT")
    print("=" * 72)
    print(f"Host: {hostname}")
    print(f"Timestamp (UTC): {time.strftime('%Y-%m-%d %H:%M:%S', time.gmtime())}")
    print()

    try:
        smi = run_cmd(["nvidia-smi", "--query-gpu=index,name,driver_version,memory.total,memory.used,utilization.gpu,utilization.memory,temperature.gpu,power.draw", "--format=csv,noheader"])
        print("--- nvidia-smi ---")
        for line in smi.strip().splitlines():
            print(f"  {line}")
        print()
    except subprocess.CalledProcessError as e:
        print(f"nvidia-smi failed: {e.output}")

    for sec in REPORT:
        print(f"--- {sec.title} ---")
        key_w = max((len(k) for k, _ in sec.rows), default=0)
        for key, val in sec.rows:
            print(f"  {key:<{key_w}}  {val}")
        print()

    print("=" * 72)
    print("END REPORT")
    print("=" * 72)


def main() -> int:
    host = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1:11434"
    model = sys.argv[2] if len(sys.argv) > 2 else "codellama:7b"

    benchmark_gpu_compute()
    benchmark_ollama(host, model)

    hostname = subprocess.check_output(["hostname", "-f"], text=True).strip()
    print_report(hostname)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
