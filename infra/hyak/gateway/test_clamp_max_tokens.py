#!/usr/bin/env python3
"""Unit tests for OpenCode-style max_tokens overflow clamp."""

import unittest

from clamp_max_tokens import clamp_completion_budget, estimate_prompt_tokens


class ClampMaxTokensTest(unittest.TestCase):
    def test_opencode_overflow_is_clamped(self):
        # Repro: OpenCode asked for 32000 output + ~769 input on a 32768 window.
        prompt = "x" * (769 * 4)
        data = {
            "model": "deepseek-v4-flash-0731",
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 32000,
        }
        prompt_tokens = estimate_prompt_tokens(data["messages"])
        self.assertGreaterEqual(prompt_tokens, 700)
        out = clamp_completion_budget(data, prompt_tokens=769)
        self.assertLessEqual(769 + out["max_tokens"], 32768)
        self.assertLess(out["max_tokens"], 32000)
        self.assertGreater(out["max_tokens"], 0)

    def test_small_request_unchanged(self):
        data = {
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 64,
        }
        out = clamp_completion_budget(data, prompt_tokens=8)
        self.assertEqual(out["max_tokens"], 64)

    def test_max_completion_tokens_clamped(self):
        data = {
            "messages": [{"role": "user", "content": "hi"}],
            "max_completion_tokens": 40000,
        }
        out = clamp_completion_budget(data, prompt_tokens=10)
        self.assertLessEqual(10 + out["max_completion_tokens"], 32768)


if __name__ == "__main__":
    unittest.main()
