#!/usr/bin/env python3
"""
对 missing-zh-CN 列表做“去噪/分类”。

输入文件支持两种格式：
  1) 原始日志行：
     ... Missing zh-CN translation: "..." (orig="...")
  2) 纯 key 列表：每行一个字符串

输出（默认写到当前工作目录）：
  - missing-zh-CN.cleaned.txt        需要翻译/保留的 key
  - missing-zh-CN.filtered.txt       被过滤掉的 key（通常不翻）
  - missing-zh-CN.categories.json    分类结果（category -> keys）

注意：这是启发式工具，目标是“实用优先”，不是 100% 精准。
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Dict, List, Tuple


LOG_RE = re.compile(r'Missing zh-CN translation: (?P<q>"(?:[^"\\]|\\.)*")')

MONTHS = [
    "jan",
    "feb",
    "mar",
    "apr",
    "may",
    "jun",
    "jul",
    "aug",
    "sep",
    "oct",
    "nov",
    "dec",
]

# 粗略的字体名信号词。命中这些通常就是“字体 family 名”，一般不需要翻译。
FONT_WORDS = {
    "nerd",
    "font",
    "mono",
    "sans",
    "serif",
    "condensed",
    "extracondensed",
    "semicondensed",
    "narrow",
    "display",
    "rounded",
    "compact",
    "text",
    "jetbrains",
    "roboto",
    "noto",
    "sf",
    "sfmono",
    "sfpro",
    "adwaita",
    "liberation",
    "nimbus",
    "lucida",
    "terminus",
    "urw",
    "newyork",
}

# Warp 主题名（按你日志里出现过的来）——通常也不翻译。
THEME_NAMES = {
    "aurora",
    "comets",
    "cow",
    "glass sky",
    "glitch",
    "glow",
    "holographic",
    "mono",
    "neon",
    "original",
    "starburst",
    "sticker",
    "warp 1",
    "classic 1",
    "classic 2",
    "classic 3",
}

KEY_TOKEN_WORDS = {
    "ctrl",
    "shift",
    "alt",
    "meta",
    "super",
    "cmd",
    "esc",
    "tab",
    "f3",
    "f11",
    "pageup",
    "pagedown",
    "space",
}


def parse_keys(text: str) -> List[str]:
    keys: List[str] = []
    for line in text.splitlines():
        line = line.rstrip("\n")
        m = LOG_RE.search(line)
        if m:
            q = m.group("q")
            try:
                key = json.loads(q)
            except Exception:
                key = q.strip('"')
            keys.append(key)
        else:
            s = line.strip()
            if not s:
                continue
            if (s.startswith('"') and s.endswith('"')) or (s.startswith("'") and s.endswith("'")):
                s = s[1:-1]
            keys.append(s)

    # 去重（保序）
    seen = set()
    out: List[str] = []
    for k in keys:
        if k in seen:
            continue
        seen.add(k)
        out.append(k)
    return out


def looks_like_version(s: str) -> bool:
    st = s.strip()
    # v#.##.### / 1.2.3 / v1.2.3 / v#.##.### 这种 pattern
    if re.fullmatch(r"v?#?(?:\d+|#)(?:\.(?:\d+|#)){1,4}", st):
        return True
    return False


def looks_like_time_or_reset(s: str) -> bool:
    low = s.lower()
    # 03:48 pm
    if re.fullmatch(r"\d{1,2}:\d{2}\s*(am|pm)", low.strip()):
        return True
    # Resets Jun 02 at 4:52 PM
    if low.startswith("resets ") and " at " in low:
        return True
    if any(m in low for m in MONTHS) and re.search(r"\b\d{1,2}\b", low) and (
        " at " in low or ":" in low
    ):
        return True
    return False


def looks_like_date(s: str) -> bool:
    low = s.lower().strip()
    # July 12, 2023
    if re.fullmatch(r"[a-z]{3,9}\s+\d{1,2},\s+\d{4}", low):
        return True
    return False


def looks_like_font_name(s: str) -> bool:
    low = s.lower().strip()
    # 单词 mono 既可能是主题名，也可能是字体的一部分；主题名优先处理
    if low in THEME_NAMES:
        return False
    if any(w in low for w in FONT_WORDS):
        # 过滤掉 “带 font/mono/sans/serif 等的完整句子”，字体名一般比较短且标题化
        if len(low) <= 64 and re.search(r"[A-Za-z]", s):
            return True
    return False


def looks_like_theme_name(s: str) -> bool:
    return s.lower().strip() in THEME_NAMES


def looks_like_model_id(s: str) -> bool:
    low = s.lower().strip()
    # gpt-5.4 / claude 4.6 sonnet (max) / kimi k2.6 (us-hosted)...
    if re.match(r"^(gpt|claude|gemini|kimi|minimax|qwen|glm)\b", low):
        return True
    if "(us-hosted)" in low or "(thinking)" in low or "(max" in low or "reasoning" in low:
        return True
    return False


def looks_like_key_token(s: str) -> bool:
    low = s.lower().strip()
    if low in KEY_TOKEN_WORDS:
        return True
    # Ctrl Shift 2 / Alt 3, Alt Shift F / Ctrl Shift | / ctrl- / shift-
    if re.fullmatch(r"(ctrl|shift|alt|meta|super|cmd)\s+.+", low) and len(low) <= 32:
        return True
    if re.fullmatch(r"(ctrl|shift)-", low):
        return True
    return False


def looks_like_slash_command(s: str) -> bool:
    st = s.strip()
    if st.startswith("/") and len(st) <= 80:
        if re.fullmatch(r"/[A-Za-z][A-Za-z0-9_-]*", st):
            return True
    if "slash command:" in s.lower() and "/" in s:
        return True
    return False


def categorize(s: str) -> Tuple[str, str]:
    """
    返回 (category, action)：
      - action == "keep"   -> 保留（建议翻译）
      - action == "filter" -> 过滤（通常不翻）
    """
    st = s.strip()
    if not st:
        return ("empty", "filter")

    # 如果输入文件里混入了整行日志（带时间戳/级别），直接过滤掉。
    # 例：16:52:21.051 [WARN] ...
    if re.match(r"^\d{1,2}:\d{2}:\d{2}\.\d{3}\s+\[[A-Z]+\]\s+", st):
        return ("log_line", "filter")
    # 例：2026-05-01T14:55:38Z [INFO] ...
    if re.match(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z\s+\[[A-Z]+\]\s+", st):
        return ("log_line", "filter")
    # miss collector 自己的提示行也过滤（它们不是“missing key”）
    low = st.lower()
    if "enabled missing translation collection" in low or "missing translation collection:" in low:
        return ("collector_banner", "filter")

    # 通常不翻译的噪声类
    if looks_like_font_name(st):
        return ("font_name", "filter")
    if looks_like_theme_name(st):
        return ("theme_name", "filter")
    if looks_like_model_id(st):
        return ("model_id", "filter")
    if looks_like_date(st):
        return ("date", "filter")
    if looks_like_time_or_reset(st):
        return ("time_or_reset", "filter")
    if looks_like_version(st):
        return ("version_pattern", "filter")

    # 通常要翻，但单独成类方便你后面批量处理
    if looks_like_key_token(st):
        return ("key_token", "keep")
    if looks_like_slash_command(st):
        return ("slash_command", "keep")

    # 类似 global/search/rewind 这类“短 id”，多数其实是 UI 标签，建议保留翻译
    if re.fullmatch(r"[a-z0-9_-]{2,32}", low) and not re.search(r"[A-Z]", st):
        return ("short_id", "keep")

    return ("ui_text", "keep")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("input", type=Path, help="输入文件：missing list 或原始日志")
    ap.add_argument(
        "--out-dir", type=Path, default=Path("."), help="输出目录（默认当前目录）"
    )
    args = ap.parse_args()

    text = args.input.read_text(encoding="utf-8", errors="replace")
    keys = parse_keys(text)

    categories: Dict[str, List[str]] = {}
    cleaned: List[str] = []
    filtered: List[str] = []

    for k in keys:
        cat, action = categorize(k)
        categories.setdefault(cat, []).append(k)
        if action == "keep":
            cleaned.append(k)
        else:
            filtered.append(k)

    out_dir = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "missing-zh-CN.cleaned.txt").write_text(
        "\n".join(cleaned) + ("\n" if cleaned else ""), encoding="utf-8"
    )
    (out_dir / "missing-zh-CN.filtered.txt").write_text(
        "\n".join(filtered) + ("\n" if filtered else ""), encoding="utf-8"
    )
    (out_dir / "missing-zh-CN.categories.json").write_text(
        json.dumps(categories, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )

    print(f"Total keys: {len(keys)}")
    print(f"Keep (translate): {len(cleaned)}")
    print(f"Filtered: {len(filtered)}")
    for cat in sorted(categories.keys()):
        print(f"- {cat}: {len(categories[cat])}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
