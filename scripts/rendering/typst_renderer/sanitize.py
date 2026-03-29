from __future__ import annotations

import json
import os
import re
from pathlib import Path

from config import fonts
from rendering.render_payload_parts.shared import get_render_protected_text
from rendering.typst_renderer.compiler import compile_typst_overlay_pdf
from rendering.typst_renderer.shared import TYPST_OVERLAY_DIR
from rendering.typst_renderer.shared import force_plain_text_item_at_index
from rendering.typst_renderer.shared import force_plain_text_items
from rendering.typst_renderer.shared import strip_formula_commands_for_item_at_index
from translation.llm.deepseek_client import request_chat_content


TYPST_REPAIR_MODEL_ENV = "TYPST_REPAIR_MODEL"
TYPST_REPAIR_BASE_URL_ENV = "TYPST_REPAIR_BASE_URL"
TYPST_REPAIR_ENABLED_ENV = "TYPST_REPAIR_LLM_ENABLED"
_FENCED_BLOCK_RE = re.compile(r"^\s*```[^\n]*\n(?P<body>.*)\n```\s*$", re.DOTALL)
_PROTECTED_TEXT_BLOCK_RE = re.compile(
    r"<<<PROTECTED_TEXT>>>\s*(?P<content>.*?)\s*<<<END_PROTECTED_TEXT>>>",
    re.DOTALL,
)
_FORMULA_BLOCK_RE = re.compile(
    r"<<<FORMULA\s+placeholder=(?P<placeholder>\[\[FORMULA_\d+]])\s*>>>\s*"
    r"(?P<content>.*?)"
    r"\s*<<<END_FORMULA>>>",
    re.DOTALL,
)


def _typst_repair_enabled() -> bool:
    value = os.environ.get(TYPST_REPAIR_ENABLED_ENV, "1")
    return value.strip().lower() not in {"0", "false", "no", "off"}


def _item_formula_map(item: dict) -> list[dict]:
    formula_map = item.get("render_formula_map") or item.get("translation_unit_formula_map") or item.get("formula_map", [])
    normalized: list[dict] = []
    for entry in formula_map:
        placeholder = str(entry.get("placeholder", "") or "").strip()
        formula_text = str(entry.get("formula_text", "") or "").strip()
        if not placeholder:
            continue
        normalized.append({"placeholder": placeholder, "formula_text": formula_text})
    return normalized


def _item_render_text(item: dict) -> str:
    return get_render_protected_text(item)


def _apply_formula_map(item: dict, formula_map: list[dict]) -> dict:
    cloned = dict(item)
    if "render_formula_map" in cloned:
        cloned["render_formula_map"] = formula_map
    if "translation_unit_formula_map" in cloned:
        cloned["translation_unit_formula_map"] = formula_map
    cloned["formula_map"] = formula_map
    return cloned


def _strip_wrapping_fence(text: str) -> str:
    stripped = (text or "").strip()
    match = _FENCED_BLOCK_RE.match(stripped)
    if match:
        return (match.group("body") or "").strip()
    return stripped


def _parse_typst_repair_response(
    content: str,
    *,
    original_protected_text: str,
    formula_map: list[dict],
) -> tuple[str, list[dict]]:
    text = _strip_wrapping_fence(content)
    protected_match = _PROTECTED_TEXT_BLOCK_RE.search(text)
    repaired_text = (
        (protected_match.group("content") or "").strip()
        if protected_match is not None
        else original_protected_text
    )

    repaired_lookup: dict[str, str] = {}
    for match in _FORMULA_BLOCK_RE.finditer(text):
        placeholder = str(match.group("placeholder") or "").strip()
        formula_text = str(match.group("content") or "").strip()
        if placeholder and formula_text:
            repaired_lookup[placeholder] = formula_text

    repaired_formula_map: list[dict] = []
    for entry in formula_map:
        placeholder = str(entry.get("placeholder", "") or "").strip()
        original_formula_text = str(entry.get("formula_text", "") or "").strip()
        repaired_formula_map.append(
            {
                "placeholder": placeholder,
                "formula_text": repaired_lookup.get(placeholder, original_formula_text),
            }
        )

    original_placeholders = [entry["placeholder"] for entry in formula_map if entry.get("placeholder")]
    if any(placeholder not in repaired_text for placeholder in original_placeholders):
        repaired_text = original_protected_text

    return repaired_text, repaired_formula_map


def _resolve_typst_repair_request(
    *,
    api_key: str,
    model: str,
    base_url: str,
) -> tuple[str, str, str] | None:
    if not _typst_repair_enabled():
        return None
    resolved_api_key = (api_key or "").strip()
    resolved_model = (os.environ.get(TYPST_REPAIR_MODEL_ENV, model) or "").strip()
    resolved_base_url = (os.environ.get(TYPST_REPAIR_BASE_URL_ENV, base_url) or "").strip()
    if not (resolved_api_key and resolved_model and resolved_base_url):
        return None
    return resolved_api_key, resolved_model, resolved_base_url


def _repair_item_with_llm_for_typst(
    item: dict,
    *,
    request_label: str,
    api_key: str,
    model: str,
    base_url: str,
) -> dict:
    repair_request = _resolve_typst_repair_request(
        api_key=api_key,
        model=model,
        base_url=base_url,
    )
    if repair_request is None:
        return item
    repair_api_key, repair_model, repair_base_url = repair_request

    protected_text = _item_render_text(item)
    formula_map = _item_formula_map(item)
    if not protected_text and not formula_map:
        return item

    messages = [
        {
            "role": "system",
            "content": (
                "You repair translated scientific text so it compiles in Typst/mitex.\n"
                "Keep meaning unchanged.\n"
                "Preserve Chinese wording unless needed for syntax repair.\n"
                "Do not translate, summarize, or explain.\n"
                "Keep every formula placeholder exactly unchanged.\n"
                "Only simplify broken or legacy LaTeX style commands inside formula_text, such as "
                "\\bf, \\rm, \\it, \\pmb, \\textcircled wrappers, redundant style wrappers, "
                "and other purely stylistic commands that often break Typst.\n"
                "Do not drop structural math commands like \\frac, \\sqrt, \\left, \\right, subscripts, superscripts.\n"
                "Do not return JSON, markdown, code fences, or explanations.\n"
                "Return only tagged blocks in this exact format:\n"
                "<<<PROTECTED_TEXT>>>\n"
                "repaired protected text with placeholders unchanged\n"
                "<<<END_PROTECTED_TEXT>>>\n"
                "Then return zero or more formula blocks:\n"
                "<<<FORMULA placeholder=[[FORMULA_1]]>>>\n"
                "repaired formula text\n"
                "<<<END_FORMULA>>>\n"
                "Return one formula block per placeholder only when formula_text exists."
            ),
        },
        {
            "role": "user",
            "content": json.dumps(
                {
                    "task": "Repair this translated block for Typst rendering while preserving placeholders.",
                    "item_id": item.get("item_id", ""),
                    "protected_text": protected_text,
                    "formula_map": formula_map,
                },
                ensure_ascii=False,
            ),
        },
    ]
    try:
        content = request_chat_content(
            messages,
            api_key=repair_api_key,
            model=repair_model,
            base_url=repair_base_url,
            temperature=0.0,
            response_format=None,
            timeout=60,
            request_label=request_label,
        )
    except Exception as exc:
        print(f"{request_label}: llm repair skipped: {type(exc).__name__}: {exc}", flush=True)
        return item

    repaired_text, repaired_formula_map = _parse_typst_repair_response(
        content,
        original_protected_text=protected_text,
        formula_map=formula_map,
    )

    cloned = _apply_formula_map(item, repaired_formula_map)
    if "render_protected_text" in cloned:
        cloned["render_protected_text"] = repaired_text
    if "translation_unit_protected_translated_text" in cloned:
        cloned["translation_unit_protected_translated_text"] = repaired_text
    if "protected_translated_text" in cloned:
        cloned["protected_translated_text"] = repaired_text
    return cloned


def _repair_items_with_llm_for_typst(
    translated_items: list[dict],
    bad_indices: list[int],
    *,
    stem: str,
    api_key: str,
    model: str,
    base_url: str,
) -> list[dict]:
    patched: list[dict] = []
    bad_index_set = set(bad_indices)
    for index, item in enumerate(translated_items):
        if index not in bad_index_set:
            patched.append(item)
            continue
        patched.append(
            _repair_item_with_llm_for_typst(
                item,
                request_label=f"typst-llm-repair {stem} {item.get('item_id', index)}",
                api_key=api_key,
                model=model,
                base_url=base_url,
            )
        )
    return patched


def sanitize_items_for_typst_compile(
    page_width: float,
    page_height: float,
    translated_items: list[dict],
    stem: str,
    api_key: str = "",
    model: str = "",
    base_url: str = "",
    font_family: str = fonts.TYPST_DEFAULT_FONT_FAMILY,
    include_cover_rect: bool = True,
    font_paths: list[Path] | None = None,
    work_dir: Path | None = None,
) -> list[dict]:
    work_dir = work_dir or TYPST_OVERLAY_DIR
    try:
        compile_typst_overlay_pdf(
            page_width,
            page_height,
            translated_items,
            stem=stem,
            font_family=font_family,
            include_cover_rect=include_cover_rect,
            font_paths=font_paths,
            work_dir=work_dir,
        )
        return translated_items
    except RuntimeError as page_error:
        bad_indices: list[int] = []
        for index in range(len(translated_items)):
            try:
                compile_typst_overlay_pdf(
                    page_width,
                    page_height,
                    [translated_items[index]],
                    stem=f"{stem}-probe-{index:03d}",
                    font_family=font_family,
                    include_cover_rect=include_cover_rect,
                    font_paths=font_paths,
                    work_dir=work_dir,
                )
            except RuntimeError:
                bad_indices.append(index)

        if bad_indices:
            print(f"typst selective fallback: {stem} block_indices={bad_indices}", flush=True)
            patched_items = translated_items
            for index in bad_indices:
                patched_items = strip_formula_commands_for_item_at_index(patched_items, index)
            try:
                compile_typst_overlay_pdf(
                    page_width,
                    page_height,
                    patched_items,
                    stem=f"{stem}-selective-strip",
                    font_family=font_family,
                    include_cover_rect=include_cover_rect,
                    font_paths=font_paths,
                    work_dir=work_dir,
                )
                return patched_items
            except RuntimeError:
                pass

            llm_patched_items = _repair_items_with_llm_for_typst(
                patched_items,
                bad_indices,
                stem=stem,
                api_key=api_key,
                model=model,
                base_url=base_url,
            )
            if llm_patched_items != patched_items:
                try:
                    compile_typst_overlay_pdf(
                        page_width,
                        page_height,
                        llm_patched_items,
                        stem=f"{stem}-selective-llm",
                        font_family=font_family,
                        include_cover_rect=include_cover_rect,
                        font_paths=font_paths,
                        work_dir=work_dir,
                    )
                    return llm_patched_items
                except RuntimeError:
                    pass

            patched_items = translated_items
            for index in bad_indices:
                patched_items = force_plain_text_item_at_index(patched_items, index)
            try:
                compile_typst_overlay_pdf(
                    page_width,
                    page_height,
                    patched_items,
                    stem=f"{stem}-selective-plain",
                    font_family=font_family,
                    include_cover_rect=include_cover_rect,
                    font_paths=font_paths,
                    work_dir=work_dir,
                )
                return patched_items
            except RuntimeError:
                pass

        print(f"typst page fallback to plain text: {stem}", flush=True)
        print(str(page_error), flush=True)
        patched_items = force_plain_text_items(translated_items)
        compile_typst_overlay_pdf(
            page_width,
            page_height,
            patched_items,
            stem=f"{stem}-plain",
            font_family=font_family,
            include_cover_rect=include_cover_rect,
            font_paths=font_paths,
            work_dir=work_dir,
        )
        return patched_items


def compile_overlay_pdf_resilient(
    page_width: float,
    page_height: float,
    translated_items: list[dict],
    stem: str,
    api_key: str = "",
    model: str = "",
    base_url: str = "",
    font_family: str = fonts.TYPST_DEFAULT_FONT_FAMILY,
    include_cover_rect: bool = True,
    font_paths: list[Path] | None = None,
    work_dir: Path | None = None,
) -> Path:
    work_dir = work_dir or TYPST_OVERLAY_DIR
    sanitized_items = sanitize_items_for_typst_compile(
        page_width,
        page_height,
        translated_items,
        stem=stem,
        api_key=api_key,
        model=model,
        base_url=base_url,
        font_family=font_family,
        include_cover_rect=include_cover_rect,
        font_paths=font_paths,
        work_dir=work_dir,
    )
    return compile_typst_overlay_pdf(
        page_width,
        page_height,
        sanitized_items,
        stem=f"{stem}-final",
        font_family=font_family,
        include_cover_rect=include_cover_rect,
        font_paths=font_paths,
        work_dir=work_dir,
    )


def sanitize_page_specs_for_typst_book_background(
    page_specs: list[tuple[int, float, float, list[dict]]],
    stem: str,
    api_key: str = "",
    model: str = "",
    base_url: str = "",
    font_family: str = fonts.TYPST_DEFAULT_FONT_FAMILY,
    font_paths: list[Path] | None = None,
    work_dir: Path | None = None,
) -> list[tuple[int, float, float, list[dict]]]:
    work_dir = work_dir or TYPST_OVERLAY_DIR
    sanitized_specs: list[tuple[int, float, float, list[dict]]] = []
    for page_index, (source_page_idx, page_width, page_height, translated_items) in enumerate(page_specs):
        sanitized_items = sanitize_items_for_typst_compile(
            page_width,
            page_height,
            translated_items,
            stem=f"{stem}-page-{page_index:03d}",
            api_key=api_key,
            model=model,
            base_url=base_url,
            font_family=font_family,
            include_cover_rect=True,
            font_paths=font_paths,
            work_dir=work_dir,
        )
        sanitized_specs.append((source_page_idx, page_width, page_height, sanitized_items))
    return sanitized_specs


def sanitize_page_specs_for_typst_book_overlay(
    page_specs: list[tuple[int, float, float, list[dict], str]],
    api_key: str = "",
    model: str = "",
    base_url: str = "",
    font_family: str = fonts.TYPST_DEFAULT_FONT_FAMILY,
    font_paths: list[Path] | None = None,
    work_dir: Path | None = None,
) -> list[tuple[int, float, float, list[dict], str]]:
    work_dir = work_dir or TYPST_OVERLAY_DIR
    sanitized_specs: list[tuple[int, float, float, list[dict], str]] = []
    for page_idx, page_width, page_height, translated_items, page_stem in page_specs:
        sanitized_items = sanitize_items_for_typst_compile(
            page_width,
            page_height,
            translated_items,
            stem=page_stem,
            api_key=api_key,
            model=model,
            base_url=base_url,
            font_family=font_family,
            font_paths=font_paths,
            work_dir=work_dir / "page-overlays" / page_stem,
        )
        sanitized_specs.append((page_idx, page_width, page_height, sanitized_items, page_stem))
    return sanitized_specs
