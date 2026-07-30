#!/usr/bin/env python3
"""Minimal GPU compute smoke test — run on a compute node, never login."""

import sys

try:
    import torch
except ImportError:
    print("FAIL: torch not installed")
    sys.exit(1)

if not torch.cuda.is_available():
    print("FAIL: CUDA not available")
    sys.exit(1)

n = torch.cuda.device_count()
print(f"CUDA devices: {n}")
for i in range(n):
    props = torch.cuda.get_device_properties(i)
    print(f"  [{i}] {props.name}  {props.total_memory // (1024**3)} GiB")

x = torch.randn(4096, 4096, device="cuda")
y = x @ x
torch.cuda.synchronize()
print(f"GPU matmul OK: shape={tuple(y.shape)}  mean={y.mean().item():.4f}")
print("PASS")
