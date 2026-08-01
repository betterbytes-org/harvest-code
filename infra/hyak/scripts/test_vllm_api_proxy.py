#!/usr/bin/env python3
"""Unit tests for vLLM API proxy rate limiter."""

import time
import unittest
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from vllm_api_proxy import RateLimiter, TokenBucket


class TokenBucketTests(unittest.TestCase):
    def test_allows_burst_then_throttles(self) -> None:
        bucket = TokenBucket(rate_per_minute=60.0, burst=2.0)
        self.assertTrue(bucket.allow())
        self.assertTrue(bucket.allow())
        self.assertFalse(bucket.allow())

    def test_refills_over_time(self) -> None:
        bucket = TokenBucket(rate_per_minute=6000.0, burst=1.0)
        self.assertTrue(bucket.allow())
        self.assertFalse(bucket.allow())
        time.sleep(0.02)
        self.assertTrue(bucket.allow())


class RateLimiterTests(unittest.TestCase):
    def test_per_client_isolation(self) -> None:
        limiter = RateLimiter(rpm=60.0, burst=1.0)
        self.assertTrue(limiter.allow("client-a"))
        self.assertFalse(limiter.allow("client-a"))
        self.assertTrue(limiter.allow("client-b"))


if __name__ == "__main__":
    unittest.main()
