"""LiteLLM pre-call hook: clamp max_tokens so prompt + output fit vLLM context."""

from __future__ import annotations

import os
from typing import Any, Dict, Optional

# Matches --max-model-len in start_vllm_background.sh
DEFAULT_MAX_CONTEXT = 32768
SAFETY_TOKENS = 16


def max_context_tokens() -> int:
    raw = os.environ.get("VLLM_MAX_MODEL_LEN", str(DEFAULT_MAX_CONTEXT))
    try:
        return max(1, int(raw))
    except ValueError:
        return DEFAULT_MAX_CONTEXT


def estimate_prompt_tokens(messages: Any) -> int:
    text_parts = []
    if isinstance(messages, list):
        for msg in messages:
            if not isinstance(msg, dict):
                continue
            content = msg.get("content", "")
            if isinstance(content, str):
                text_parts.append(content)
            elif isinstance(content, list):
                for part in content:
                    if isinstance(part, dict) and isinstance(part.get("text"), str):
                        text_parts.append(part["text"])
                    elif isinstance(part, str):
                        text_parts.append(part)
    text = "\n".join(text_parts)
    return max(1, (len(text) + 3) // 4)


def clamp_completion_budget(data: Dict[str, Any], prompt_tokens: Optional[int] = None) -> Dict[str, Any]:
    """Lower max_tokens / max_completion_tokens so they fit in the context window."""
    if prompt_tokens is None:
        prompt_tokens = estimate_prompt_tokens(data.get("messages"))
    room = max(1, max_context_tokens() - int(prompt_tokens) - SAFETY_TOKENS)
    for key in ("max_tokens", "max_completion_tokens"):
        val = data.get(key)
        if val is None:
            continue
        try:
            requested = int(val)
        except (TypeError, ValueError):
            continue
        if requested > room:
            data[key] = room
    return data


try:
    from litellm.integrations.custom_logger import CustomLogger

    class ClampMaxTokensHandler(CustomLogger):
        async def async_pre_call_hook(self, user_api_key_dict, cache, data, call_type):
            if not isinstance(data, dict):
                return data
            prompt_tokens = None
            try:
                import litellm

                messages = data.get("messages")
                if messages:
                    prompt_tokens = litellm.token_counter(
                        model=data.get("model") or "deepseek-v4-flash-0731",
                        messages=messages,
                    )
            except Exception:
                prompt_tokens = None
            return clamp_completion_budget(data, prompt_tokens=prompt_tokens)

    proxy_handler_instance = ClampMaxTokensHandler()
except ImportError:  # unit tests without litellm installed
    proxy_handler_instance = None
