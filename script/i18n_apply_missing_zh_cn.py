#!/usr/bin/env python3
"""
把 missing-zh-CN.cleaned.txt 里“缺失且合理”的 key 自动补到 app/src/i18n/zh_cn.json。

策略（尽量保守，避免误翻译用户内容）：
1) 只处理纯 ASCII key（miss collector 本身也会过滤非 ASCII）
2) 过滤掉明显不该翻译的项（字体名/主题名/模型名/日期时间/版本号）——与 denoise 工具一致
3) 对以下类型给出“合理默认翻译”：
   - key_token: Ctrl/Shift/Alt/Meta/Super/Cmd/Tab/Esc/Space/Pageup/Pagedown/F3/F11
   - slash_command: "/FORK" "/remote-control" 这类：保持原样（通常不翻），但为了减少 missing 噪声，
     我们仍然写入映射：key -> key（即原文不变）
   - ui_text / short_id: 这些无法可靠自动翻译，默认不写（需要人工翻译）

输出：
  - 会直接修改 app/src/i18n/zh_cn.json（保持 JSON 格式化）
  - 同时写一个 report: missing-zh-CN.apply-report.json

用法：
  python3 script/i18n_apply_missing_zh_cn.py missing-zh-CN.cleaned.txt
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Dict, List, Tuple


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


def read_keys(p: Path) -> List[str]:
    keys: List[str] = []
    for line in p.read_text(encoding="utf-8", errors="replace").splitlines():
        s = line.strip()
        if not s:
            continue
        keys.append(s)
    # unique preserve order
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
    return bool(re.fullmatch(r"v?#?(?:\d+|#)(?:\.(?:\d+|#)){1,4}", st))


def looks_like_time_or_reset(s: str) -> bool:
    low = s.lower()
    if re.fullmatch(r"\d{1,2}:\d{2}\s*(am|pm)", low.strip()):
        return True
    if low.startswith("resets ") and " at " in low:
        return True
    if any(m in low for m in MONTHS) and re.search(r"\b\d{1,2}\b", low) and (
        " at " in low or ":" in low
    ):
        return True
    return False


def looks_like_date(s: str) -> bool:
    low = s.lower().strip()
    return bool(re.fullmatch(r"[a-z]{3,9}\s+\d{1,2},\s+\d{4}", low))


def looks_like_font_name(s: str) -> bool:
    low = s.lower().strip()
    if low in THEME_NAMES:
        return False
    if any(w in low for w in FONT_WORDS):
        if len(low) <= 64 and re.search(r"[A-Za-z]", s):
            return True
    return False


def looks_like_theme_name(s: str) -> bool:
    return s.lower().strip() in THEME_NAMES


def looks_like_model_id(s: str) -> bool:
    low = s.lower().strip()
    if re.match(r"^(gpt|claude|gemini|kimi|minimax|qwen|glm)\b", low):
        return True
    if "(us-hosted)" in low or "(thinking)" in low or "(max" in low or "reasoning" in low:
        return True
    return False


def categorize_for_apply(s: str) -> str:
    st = s.strip()
    low = st.lower()
    # filter group
    if looks_like_font_name(st):
        return "font_name"
    if looks_like_theme_name(st):
        return "theme_name"
    if looks_like_model_id(st):
        return "model_id"
    if looks_like_date(st):
        return "date"
    if looks_like_time_or_reset(st):
        return "time_or_reset"
    if looks_like_version(st):
        return "version_pattern"
    # tokens
    if re.fullmatch(r"/[A-Za-z][A-Za-z0-9_-]*", st) and len(st) <= 80:
        return "slash_command"
    if low in {
        "ctrl",
        "shift",
        "alt",
        "meta",
        "super",
        "cmd",
        "esc",
        "tab",
        "space",
        "pageup",
        "pagedown",
        "f3",
        "f11",
    }:
        return "key_token"
    if re.fullmatch(r"(ctrl|shift|alt|meta|super|cmd)\s+.+", low) and len(low) <= 32:
        return "key_token_combo"
    return "ui_text"


KEY_TOKEN_TRANSLATIONS: Dict[str, str] = {
    "Ctrl": "Ctrl",
    "Shift": "Shift",
    "Alt": "Alt",
    "Meta": "Meta",
    "Super": "Super",
    "Cmd": "Cmd",
    "esc": "Esc",
    "tab": "Tab",
    "Space": "空格",
    "Pageup": "PageUp",
    "Pagedown": "PageDown",
    "F3": "F3",
    "F11": "F11",
}


def translate_key_token(s: str) -> str | None:
    # keep as-is for most modifier tokens; translate Space to 中文更友好
    # We normalize some common casing variants.
    st = s.strip()
    # exact first
    if st in KEY_TOKEN_TRANSLATIONS:
        return KEY_TOKEN_TRANSLATIONS[st]
    # common lowercase variants in logs
    low = st.lower()
    if low == "ctrl":
        return "Ctrl"
    if low == "shift":
        return "Shift"
    if low == "alt":
        return "Alt"
    if low == "meta":
        return "Meta"
    if low == "super":
        return "Super"
    if low == "cmd":
        return "Cmd"
    if low == "esc":
        return "Esc"
    if low == "tab":
        return "Tab"
    if low == "space":
        return "空格"
    if low == "pageup":
        return "PageUp"
    if low == "pagedown":
        return "PageDown"
    if low == "f3":
        return "F3"
    if low == "f11":
        return "F11"
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("input", type=Path, help="missing-zh-CN.cleaned.txt")
    ap.add_argument(
        "--map",
        type=Path,
        default=Path("app/src/i18n/zh_cn.json"),
        help="zh_cn.json 路径",
    )
    ap.add_argument(
        "--report",
        type=Path,
        default=Path("missing-zh-CN.apply-report.json"),
        help="report 输出路径",
    )
    args = ap.parse_args()

    keys = read_keys(args.input)
    mappath = args.map
    mapping: Dict[str, str] = json.loads(mappath.read_text(encoding="utf-8"))

    added: Dict[str, str] = {}
    skipped: Dict[str, List[str]] = {}

    for k in keys:
        if not k.isascii():
            skipped.setdefault("non_ascii", []).append(k)
            continue
        if k in mapping:
            skipped.setdefault("already_present", []).append(k)
            continue

        cat = categorize_for_apply(k)

        if cat in {
            "font_name",
            "theme_name",
            "model_id",
            "date",
            "time_or_reset",
            "version_pattern",
        }:
            skipped.setdefault(cat, []).append(k)
            continue

        if cat == "slash_command":
            # slash command 名称通常不需要翻译；但写入 identity 映射可以降低 missing 噪声
            mapping[k] = k
            added[k] = k
            continue

        if cat in {"key_token", "key_token_combo"}:
            t = translate_key_token(k)
            if t is None:
                skipped.setdefault("key_token_unhandled", []).append(k)
                continue
            mapping[k] = t
            added[k] = t
            continue

        # ui_text: 不自动翻译，避免胡翻
        skipped.setdefault("needs_human_translation", []).append(k)

    # stable sort keys for readability
    out = {k: mapping[k] for k in sorted(mapping.keys())}
    mappath.write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    report = {
        "input": str(args.input),
        "map": str(mappath),
        "total_input_keys": len(keys),
        "added_count": len(added),
        "added": added,
        "skipped": skipped,
    }
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    print(f"Input keys: {len(keys)}")
    print(f"Added: {len(added)}")
    print(f"Skipped buckets: {len(skipped)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

