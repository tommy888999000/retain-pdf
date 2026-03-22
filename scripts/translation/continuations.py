import re


TERMINAL_PUNCTUATION = (".", "!", "?", ":", ";")
LOWER_START_RE = re.compile(r"^[a-z]")
UPPER_START_RE = re.compile(r"^[A-Z]")
HEADING_START_RE = re.compile(r"^(?:\(?\d+(?:\.\d+)*\)?[.)]?\s+|[A-Z][A-Z\s\-]{3,}|[•\-*]\s+)")
SOFT_BREAK_PUNCTUATION = (",",)
CONTINUATION_START_WORDS = {
    "and",
    "or",
    "but",
    "with",
    "without",
    "whereas",
    "while",
    "which",
    "that",
    "than",
    "then",
    "thus",
    "therefore",
    "however",
    "nevertheless",
    "moreover",
    "furthermore",
    "second",
}
CONTINUATION_END_WORDS = {
    "the",
    "a",
    "an",
    "of",
    "to",
    "for",
    "with",
    "and",
    "or",
    "but",
    "that",
    "these",
    "those",
    "this",
    "two",
    "three",
    "four",
    "five",
    "several",
    "many",
    "more",
    "less",
}
SUSPICIOUS_END_WORDS = {
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "have",
    "has",
    "had",
    "can",
    "could",
    "may",
    "might",
    "should",
    "would",
    "must",
    "will",
    "shall",
}


def _normalize(text: str) -> str:
    return " ".join((text or "").split())


def _last_word(text: str) -> str:
    tokens = re.findall(r"[A-Za-z]+(?:[-'][A-Za-z]+)?", text)
    return tokens[-1].lower() if tokens else ""


def _starts_like_continuation(text: str) -> bool:
    stripped = _normalize(text)
    if not stripped:
        return False
    if LOWER_START_RE.match(stripped):
        return True
    first = _last_word(stripped[:32])
    return first in CONTINUATION_START_WORDS


def _ends_like_continuation(text: str) -> bool:
    stripped = _normalize(text)
    if not stripped:
        return False
    if stripped.endswith("-"):
        return True
    if stripped.endswith(TERMINAL_PUNCTUATION):
        return False
    last = _last_word(stripped)
    return last in CONTINUATION_END_WORDS


def _ends_with_soft_break(text: str) -> bool:
    stripped = _normalize(text)
    return bool(stripped) and stripped.endswith(SOFT_BREAK_PUNCTUATION)


def _starts_like_heading_or_list(text: str) -> bool:
    stripped = _normalize(text)
    return bool(stripped) and bool(HEADING_START_RE.match(stripped))


def _starts_with_upper(text: str) -> bool:
    stripped = _normalize(text)
    return bool(stripped) and bool(UPPER_START_RE.match(stripped))


def _last_token_is_suspicious(text: str) -> bool:
    return _last_word(text) in SUSPICIOUS_END_WORDS


def _bbox(item: dict) -> list[float]:
    bbox = item.get("bbox", [])
    return bbox if len(bbox) == 4 else []


def _column_gap(prev_bbox: list[float], next_bbox: list[float]) -> float:
    if not prev_bbox or not next_bbox:
        return 0.0
    return next_bbox[0] - prev_bbox[2]


def _vertical_gap(prev_bbox: list[float], next_bbox: list[float]) -> float:
    if not prev_bbox or not next_bbox:
        return 0.0
    return next_bbox[1] - prev_bbox[3]


def _same_page(a: dict, b: dict) -> bool:
    return a.get("page_idx") == b.get("page_idx")


def _eligible(item: dict) -> bool:
    return item.get("block_type") == "text" and bool(_normalize(item.get("protected_source_text", "")))


def _clear_continuation_state(item: dict) -> None:
    item["continuation_group"] = ""
    item["continuation_prev_text"] = ""
    item["continuation_next_text"] = ""
    item["continuation_decision"] = ""
    item["continuation_candidate_prev_id"] = ""
    item["continuation_candidate_next_id"] = ""


def _same_column(prev_bbox: list[float], next_bbox: list[float]) -> bool:
    if not prev_bbox or not next_bbox:
        return False
    return abs(next_bbox[0] - prev_bbox[0]) <= 28


def _likely_pair_geometry(prev_item: dict, next_item: dict) -> bool:
    prev_bbox = _bbox(prev_item)
    next_bbox = _bbox(next_item)
    if not prev_bbox or not next_bbox:
        return True
    if _same_page(prev_item, next_item):
        cross_column = next_bbox[0] > prev_bbox[2] + 8
        near_vertical = _same_column(prev_bbox, next_bbox) and _vertical_gap(prev_bbox, next_bbox) <= 40
        if cross_column:
            return _column_gap(prev_bbox, next_bbox) <= 96
        return near_vertical
    return True


def _pair_join_score(prev_item: dict, next_item: dict) -> int:
    prev_page_idx = prev_item.get("page_idx", -1)
    next_page_idx = next_item.get("page_idx", -1)
    if next_page_idx < prev_page_idx or next_page_idx - prev_page_idx > 1:
        return -999
    if not _eligible(prev_item) or not _eligible(next_item):
        return -999
    prev_text = _normalize(prev_item.get("protected_source_text", ""))
    next_text = _normalize(next_item.get("protected_source_text", ""))
    if not prev_text or not next_text:
        return -999

    score = 0
    if _starts_like_continuation(next_text):
        score += 3
    if _ends_like_continuation(prev_text):
        score += 3
    if prev_text.endswith("-"):
        score += 4
    if _ends_with_soft_break(prev_text):
        score += 1
    if _last_token_is_suspicious(prev_text):
        score += 1
    if next_page_idx != prev_page_idx:
        if not prev_text.endswith(TERMINAL_PUNCTUATION):
            score += 2
    elif _likely_pair_geometry(prev_item, next_item):
        score += 1
    return score


def _pair_break_score(prev_item: dict, next_item: dict) -> int:
    prev_text = _normalize(prev_item.get("protected_source_text", ""))
    next_text = _normalize(next_item.get("protected_source_text", ""))
    score = 0
    if prev_text.endswith((".", "!", "?")):
        score += 4
    elif prev_text.endswith(TERMINAL_PUNCTUATION):
        score += 2
    if _starts_like_heading_or_list(next_text):
        score += 3
    if _starts_with_upper(next_text) and not _starts_like_continuation(next_text):
        score += 1
    prev_bbox = _bbox(prev_item)
    next_bbox = _bbox(next_item)
    if _same_page(prev_item, next_item) and prev_bbox and next_bbox:
        if not _likely_pair_geometry(prev_item, next_item):
            score += 2
    return score


def _pair_decision(prev_item: dict, next_item: dict) -> str:
    join_score = _pair_join_score(prev_item, next_item)
    if join_score < 0:
        return "break"
    break_score = _pair_break_score(prev_item, next_item)
    if join_score >= 4 and join_score >= break_score + 2:
        return "join"
    if break_score >= 4 and break_score >= join_score + 1:
        return "break"
    return "candidate"


def annotate_continuation_context(payload: list[dict]) -> int:
    for item in payload:
        _clear_continuation_state(item)

    group_index = 0
    annotated = 0
    i = 0

    def next_candidate_index(start: int) -> int | None:
        if start >= len(payload):
            return None
        current_page_idx = payload[start - 1].get("page_idx", -1) if start > 0 else payload[start].get("page_idx", -1)
        for idx in range(start, len(payload)):
            item = payload[idx]
            page_idx = item.get("page_idx", -1)
            if page_idx < current_page_idx:
                continue
            if page_idx - current_page_idx > 1:
                return None
            if _eligible(item):
                return idx
        return None

    while i < len(payload) - 1:
        current = payload[i]
        next_idx = next_candidate_index(i + 1)
        if next_idx is None:
            break
        nxt = payload[next_idx]
        decision = _pair_decision(current, nxt)
        if decision != "join":
            if decision == "candidate":
                current["continuation_decision"] = "candidate_break"
                current["continuation_candidate_next_id"] = nxt.get("item_id", "")
                nxt["continuation_decision"] = "candidate_break"
                nxt["continuation_candidate_prev_id"] = current.get("item_id", "")
            i += 1
            continue

        group_index += 1
        group_id = f"cg-{current.get('page_idx', 0) + 1:03d}-{group_index:03d}"
        chain = [current, nxt]
        j = next_idx
        while j < len(payload) - 1:
            probe_idx = next_candidate_index(j + 1)
            if probe_idx is None or _pair_decision(payload[j], payload[probe_idx]) != "join":
                break
            chain.append(payload[probe_idx])
            j = probe_idx

        for pos, item in enumerate(chain):
            item["continuation_group"] = group_id
            item["continuation_decision"] = "joined"
            if pos > 0:
                item["continuation_prev_text"] = _normalize(chain[pos - 1].get("protected_source_text", ""))
            if pos < len(chain) - 1:
                item["continuation_next_text"] = _normalize(chain[pos + 1].get("protected_source_text", ""))
            annotated += 1
        i = j + 1

    return annotated


def annotate_continuation_context_global(payloads_by_page: dict[int, list[dict]]) -> int:
    ordered_pages = sorted(payloads_by_page)
    flat_payload: list[dict] = []
    for page_idx in ordered_pages:
        flat_payload.extend(payloads_by_page[page_idx])
    return annotate_continuation_context(flat_payload)


def summarize_continuation_decisions(payload: list[dict]) -> dict[str, int]:
    summary = {
        "joined_items": 0,
        "candidate_break_items": 0,
    }
    for item in payload:
        decision = item.get("continuation_decision", "")
        if decision == "joined":
            summary["joined_items"] += 1
        elif decision == "candidate_break":
            summary["candidate_break_items"] += 1
    return summary


def candidate_continuation_pairs(payload: list[dict]) -> list[dict]:
    item_by_id = {item.get("item_id", ""): item for item in payload}
    pairs: list[dict] = []
    seen: set[tuple[str, str]] = set()
    for item in payload:
        next_id = item.get("continuation_candidate_next_id", "") or ""
        item_id = item.get("item_id", "") or ""
        if not item_id or not next_id:
            continue
        pair_key = (item_id, next_id)
        if pair_key in seen:
            continue
        next_item = item_by_id.get(next_id)
        if not next_item:
            continue
        seen.add(pair_key)
        pairs.append(
            {
                "prev_item_id": item_id,
                "next_item_id": next_id,
                "prev_text": _normalize(item.get("protected_source_text", "")),
                "next_text": _normalize(next_item.get("protected_source_text", "")),
                "prev_page_idx": item.get("page_idx", -1),
                "next_page_idx": next_item.get("page_idx", -1),
                "prev_bbox": _bbox(item),
                "next_bbox": _bbox(next_item),
            }
        )
    return pairs


def apply_candidate_pair_joins(payload: list[dict], approved_pairs: list[tuple[str, str]]) -> int:
    if not approved_pairs:
        return 0
    item_by_id = {item.get("item_id", ""): item for item in payload}
    next_map = {prev_id: next_id for prev_id, next_id in approved_pairs}
    prev_targets = {next_id for _, next_id in approved_pairs}
    starts = [prev_id for prev_id, _ in approved_pairs if prev_id not in prev_targets]
    group_index = 1000
    annotated = 0

    def assign_chain(start_id: str) -> None:
        nonlocal group_index
        chain: list[dict] = []
        current_id = start_id
        visited: set[str] = set()
        while current_id and current_id not in visited:
            visited.add(current_id)
            item = item_by_id.get(current_id)
            if not item:
                break
            chain.append(item)
            current_id = next_map.get(current_id, "")
        if len(chain) < 2:
            return
        group_index += 1
        group_id = f"cg-review-{group_index:04d}"
        for pos, item in enumerate(chain):
            item["continuation_group"] = group_id
            item["continuation_decision"] = "review_joined"
            item["continuation_candidate_prev_id"] = ""
            item["continuation_candidate_next_id"] = ""
            item["continuation_prev_text"] = _normalize(chain[pos - 1].get("protected_source_text", "")) if pos > 0 else ""
            item["continuation_next_text"] = _normalize(chain[pos + 1].get("protected_source_text", "")) if pos < len(chain) - 1 else ""

    for start_id in starts:
        before = sum(1 for item in payload if item.get("continuation_decision") == "review_joined")
        assign_chain(start_id)
        after = sum(1 for item in payload if item.get("continuation_decision") == "review_joined")
        annotated += max(0, after - before)

    return annotated
