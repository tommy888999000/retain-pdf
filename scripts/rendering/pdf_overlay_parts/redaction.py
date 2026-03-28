from __future__ import annotations

import fitz

from rendering.pdf_overlay_parts.redaction_analysis import (
    collect_page_drawing_rects,
    item_has_removable_text,
    page_has_large_background_image,
    item_should_use_cover_only,
    page_drawing_count,
    page_is_vector_heavy,
    page_is_vector_heavy_count,
    page_should_use_cover_only,
    page_should_use_cover_only_count,
)
from rendering.pdf_overlay_parts.redaction_geometry import expand_image_page_item_rect
from rendering.pdf_overlay_parts.redaction_geometry import expand_item_rect
from rendering.pdf_overlay_parts.redaction_fill import (
    apply_prepared_background_covers,
    draw_background_covers,
    draw_flat_white_covers,
    prepare_background_covers,
    draw_white_covers,
    resolved_fill_color,
)
from rendering.pdf_overlay_parts.shared import iter_valid_translated_items


def _iter_valid_redaction_items(
    translated_items: list[dict],
    *,
    image_page: bool = False,
) -> list[tuple[fitz.Rect, dict, str]]:
    redaction_items: list[tuple[fitz.Rect, dict, str]] = []
    for rect, item, translated_text in iter_valid_translated_items(translated_items):
        expanded = expand_image_page_item_rect(rect) if image_page else expand_item_rect(rect)
        if expanded.is_empty:
            continue
        redaction_items.append((expanded, item, translated_text))
    return redaction_items


def redact_translated_text_areas(
    page: fitz.Page,
    translated_items: list[dict],
    fill_background: bool | None = None,
    cover_only: bool = False,
) -> None:
    image_page = page_has_large_background_image(page)
    valid_items = _iter_valid_redaction_items(translated_items, image_page=image_page)
    if not valid_items:
        return

    if cover_only:
        draw_flat_white_covers(page, [rect for rect, _item, _translated_text in valid_items])
        return

    if image_page:
        rects = [rect for rect, _item, _translated_text in valid_items]
        prepared_covers = prepare_background_covers(page, rects)
        for rect in rects:
            page.add_redact_annot(rect, fill=False)
        page.apply_redactions(
            images=fitz.PDF_REDACT_IMAGE_PIXELS,
            graphics=fitz.PDF_REDACT_LINE_ART_REMOVE_IF_TOUCHED,
            text=fitz.PDF_REDACT_TEXT_REMOVE,
        )
        apply_prepared_background_covers(page, prepared_covers)
        return

    drawing_count = page_drawing_count(page)
    if fill_background is None and page_should_use_cover_only_count(drawing_count):
        draw_flat_white_covers(page, [rect for rect, _item, _translated_text in valid_items])
        return

    if fill_background is None and page_is_vector_heavy_count(drawing_count):
        rects = [rect for rect, _item, _translated_text in valid_items]
        draw_white_covers(page, rects)
        for rect in rects:
            page.add_redact_annot(rect, fill=False)
        page.apply_redactions(
            images=fitz.PDF_REDACT_IMAGE_PIXELS,
            graphics=fitz.PDF_REDACT_LINE_ART_REMOVE_IF_TOUCHED,
            text=fitz.PDF_REDACT_TEXT_REMOVE,
        )
        return

    drawing_rects = collect_page_drawing_rects(page)
    if fill_background is None and page_should_use_cover_only(drawing_rects):
        draw_white_covers(page, [rect for rect, _item, _translated_text in valid_items])
        return

    redactions: list[tuple[fitz.Rect, tuple[float, float, float] | None]] = []
    cover_rects: list[fitz.Rect] = []
    for rect, item, _translated_text in valid_items:
        if fill_background is None:
            removable_text = item_has_removable_text(page, item, rect)
            if removable_text:
                redactions.append((rect, None))
                continue
            if item_should_use_cover_only(rect, drawing_rects):
                cover_rects.append(rect)
                continue
            fill = (1, 1, 1)
        else:
            fill = (1, 1, 1) if fill_background else None
        redactions.append((rect, fill))

    draw_white_covers(page, cover_rects)

    for rect, fill in redactions:
        page.add_redact_annot(rect, fill=resolved_fill_color(page, rect, fill))
    if redactions:
        page.apply_redactions(
            images=fitz.PDF_REDACT_IMAGE_NONE,
            graphics=fitz.PDF_REDACT_LINE_ART_NONE,
            text=fitz.PDF_REDACT_TEXT_REMOVE,
        )


__all__ = [
    "item_has_removable_text",
    "redact_translated_text_areas",
]
