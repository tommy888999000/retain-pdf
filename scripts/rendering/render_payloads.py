import re
from dataclasses import dataclass


@dataclass
class RenderBlock:
    bbox: list[float]
    markdown_text: str


def _formula_map_lookup(formula_map: list[dict]) -> dict[str, str]:
    return {item["placeholder"]: item["formula_text"] for item in formula_map}


def _split_protected_text(protected_text: str) -> list[str]:
    parts: list[str] = []
    cursor = 0
    while cursor < len(protected_text):
        start = protected_text.find("[[FORMULA_", cursor)
        if start == -1:
            parts.append(protected_text[cursor:])
            break
        if start > cursor:
            parts.append(protected_text[cursor:start])
        end = protected_text.find("]]", start)
        if end == -1:
            parts.append(protected_text[start:])
            break
        parts.append(protected_text[start : end + 2])
        cursor = end + 2
    return [part for part in parts if part]


def _normalize_formula_for_latex_math(formula_text: str) -> str:
    expr = " ".join(formula_text.strip().split())
    if not expr:
        return expr
    expr = re.sub(r"\\begin\{array\}\s*\{[^{}]*\}\s*", "", expr)
    expr = re.sub(r"\s*\\end\{array\}", "", expr)
    expr = re.sub(r"\\cal\s+([A-Za-z])", r"\\mathcal{\1}", expr)
    if expr.startswith(("_", "^")):
        expr = "{} " + expr
    return expr


def _looks_like_citation(formula_text: str) -> bool:
    expr = " ".join(formula_text.strip().split())
    return bool(re.fullmatch(r"\[\s*\d+(?:\s*[-,]\s*\d+)*\s*\]", expr))


def _normalize_plain_citation(formula_text: str) -> str:
    digits = re.findall(r"\d+", formula_text)
    return f"[{','.join(digits)}]" if digits else formula_text.strip()


def build_markdown_paragraph(item: dict) -> str:
    protected = item.get("protected_translated_text") or item.get("protected_source_text", "")
    parts = _split_protected_text(protected)
    formula_lookup = _formula_map_lookup(item.get("formula_map", []))
    chunks: list[str] = []

    for part in parts:
        if part.startswith("[[FORMULA_"):
            formula_text = formula_lookup.get(part, part)
            if _looks_like_citation(formula_text):
                chunks.append(_normalize_plain_citation(formula_text))
                continue
            chunks.append(f"${_normalize_formula_for_latex_math(formula_text)}$")
        else:
            text = part.strip()
            if text:
                chunks.append(text)

    return "".join(chunks).strip()


def build_render_blocks(translated_items: list[dict]) -> list[RenderBlock]:
    blocks: list[RenderBlock] = []
    for item in translated_items:
        translated_text = (item.get("translated_text") or "").strip()
        bbox = item.get("bbox", [])
        if len(bbox) != 4 or not translated_text:
            continue
        blocks.append(
            RenderBlock(
                bbox=bbox,
                markdown_text=build_markdown_paragraph(item),
            )
        )
    return blocks
