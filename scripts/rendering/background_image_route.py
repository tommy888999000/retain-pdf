from __future__ import annotations

from io import BytesIO

import fitz
from PIL import Image
from PIL import ImageDraw

from rendering.pdf_overlay_parts.redaction_analysis import page_has_large_background_image
from rendering.pdf_overlay_parts.redaction_fill import quantile
from rendering.pdf_overlay_parts.redaction_geometry import rect_area
from rendering.pdf_overlay_parts.shared import iter_valid_translated_items


STRICT_VERTICAL_MERGE_GAP_PT = 2.0
STRICT_VERTICAL_MERGE_MIN_WIDTH_OVERLAP_RATIO = 0.72


def _pick_primary_background_image(
    page: fitz.Page,
    *,
    coverage_ratio_threshold: float = 0.75,
) -> tuple[int, fitz.Rect] | None:
    page_area = max(rect_area(page.rect), 1.0)
    best: tuple[float, int, fitz.Rect] | None = None
    try:
        images = page.get_images(full=True)
    except Exception:
        return None

    for image in images:
        if not image:
            continue
        xref = image[0]
        try:
            rects = page.get_image_rects(xref)
        except Exception:
            continue
        for rect in rects:
            if rect.is_empty:
                continue
            coverage_ratio = rect_area(rect & page.rect) / page_area
            if coverage_ratio < coverage_ratio_threshold:
                continue
            candidate = (coverage_ratio, xref, rect)
            if best is None or candidate[0] > best[0]:
                best = candidate
    if best is None:
        return None
    return best[1], best[2]


def _extract_image_rgb(doc: fitz.Document, xref: int) -> Image.Image | None:
    try:
        payload = doc.extract_image(xref)
    except Exception:
        payload = None
    if payload and payload.get("image"):
        try:
            image = Image.open(BytesIO(payload["image"]))
            return image.convert("RGB")
        except Exception:
            pass
    try:
        pix = fitz.Pixmap(doc, xref)
        image = Image.frombytes("RGB", [pix.width, pix.height], pix.samples)
        return image
    except Exception:
        return None


def _extract_image_payload(doc: fitz.Document, xref: int) -> dict | None:
    try:
        payload = doc.extract_image(xref)
    except Exception:
        payload = None
    return payload if payload and payload.get("image") else None


def _pdf_bool(value: str) -> bool:
    return str(value or "").strip().lower() == "true"


def _pdf_name(value: str) -> str:
    return str(value or "").strip()


def _pdf_int(value: str, default: int = 0) -> int:
    try:
        return int(str(value or "").strip())
    except Exception:
        return default


def _raw_stream_image_meta(doc: fitz.Document, xref: int) -> dict | None:
    filter_type, filter_value = doc.xref_get_key(xref, "Filter")
    width_type, width_value = doc.xref_get_key(xref, "Width")
    height_type, height_value = doc.xref_get_key(xref, "Height")
    bpc_type, bpc_value = doc.xref_get_key(xref, "BitsPerComponent")
    image_mask_type, image_mask_value = doc.xref_get_key(xref, "ImageMask")
    colorspace_type, colorspace_value = doc.xref_get_key(xref, "ColorSpace")

    if filter_type != "name" or _pdf_name(filter_value) != "/FlateDecode":
        return None

    width = _pdf_int(width_value)
    height = _pdf_int(height_value)
    bits_per_component = _pdf_int(bpc_value)
    image_mask = image_mask_type == "bool" and _pdf_bool(image_mask_value)
    color_space = _pdf_name(colorspace_value) if colorspace_type == "name" else ""

    if width <= 0 or height <= 0:
        return None
    if image_mask and bits_per_component == 1:
        return {
            "mode": "1",
            "width": width,
            "height": height,
            "fill": 1,
        }
    if bits_per_component == 8 and color_space == "/DeviceGray":
        return {
            "mode": "L",
            "width": width,
            "height": height,
            "fill": 255,
        }
    if bits_per_component == 8 and color_space == "/DeviceRGB":
        return {
            "mode": "RGB",
            "width": width,
            "height": height,
            "fill": (255, 255, 255),
        }
    return None


def _extract_raw_stream_image(doc: fitz.Document, xref: int, meta: dict) -> Image.Image | None:
    try:
        raw = doc.xref_stream(xref)
        return Image.frombytes(meta["mode"], (meta["width"], meta["height"]), raw)
    except Exception:
        return None


def _brightness_spread(pixels: list[tuple[int, int, int]]) -> int:
    if not pixels:
        return 255
    brightness = sorted(int((r + g + b) / 3) for r, g, b in pixels)
    return quantile(brightness, 9, 10) - quantile(brightness, 1, 10)


def _map_rect_to_image(image_rect: fitz.Rect, image_size: tuple[int, int], rect: fitz.Rect) -> tuple[int, int, int, int] | None:
    width, height = image_size
    if width <= 0 or height <= 0:
        return None
    inter = rect & image_rect
    if inter.is_empty:
        return None
    sx = width / max(image_rect.width, 1e-6)
    sy = height / max(image_rect.height, 1e-6)
    x0 = max(0, min(width, int((inter.x0 - image_rect.x0) * sx)))
    y0 = max(0, min(height, int((inter.y0 - image_rect.y0) * sy)))
    x1 = max(0, min(width, int((inter.x1 - image_rect.x0) * sx)))
    y1 = max(0, min(height, int((inter.y1 - image_rect.y0) * sy)))
    if x1 - x0 < 2 or y1 - y0 < 2:
        return None
    return x0, y0, x1, y1


def _width_overlap_ratio(a: fitz.Rect, b: fitz.Rect) -> float:
    overlap = max(0.0, min(a.x1, b.x1) - max(a.x0, b.x0))
    min_width = max(1e-6, min(a.width, b.width))
    return overlap / min_width


def _merge_close_vertical_rects(rects: list[fitz.Rect]) -> list[fitz.Rect]:
    if not rects:
        return []
    ordered = sorted(rects, key=lambda rect: (rect.y0, rect.x0))
    merged: list[fitz.Rect] = [fitz.Rect(ordered[0])]
    for rect in ordered[1:]:
        current = merged[-1]
        gap = rect.y0 - current.y1
        if 0.0 <= gap <= STRICT_VERTICAL_MERGE_GAP_PT and _width_overlap_ratio(current, rect) >= STRICT_VERTICAL_MERGE_MIN_WIDTH_OVERLAP_RATIO:
            merged[-1] = fitz.Rect(
                min(current.x0, rect.x0),
                min(current.y0, rect.y0),
                max(current.x1, rect.x1),
                max(current.y1, rect.y1),
            )
            continue
        merged.append(fitz.Rect(rect))
    return merged


def _candidate_strips(width: int, height: int, box: tuple[int, int, int, int]) -> list[tuple[int, int, int, int]]:
    x0, y0, x1, y1 = box
    margin = max(6, min(24, int(min(x1 - x0, y1 - y0) * 0.35)))
    candidates = [
        (max(0, x0 - margin), y0, x0, y1),
        (x1, y0, min(width, x1 + margin), y1),
        (x0, max(0, y0 - margin), x1, y0),
        (x0, y1, x1, min(height, y1 + margin)),
    ]
    return [candidate for candidate in candidates if candidate[2] - candidate[0] >= 2 and candidate[3] - candidate[1] >= 2]


def _pick_background_patch(image: Image.Image, box: tuple[int, int, int, int]) -> Image.Image | None:
    width, height = image.size
    best_patch: Image.Image | None = None
    best_score: tuple[int, int, int] | None = None
    for candidate in _candidate_strips(width, height, box):
        patch = image.crop(candidate)
        pixels = list(patch.getdata())
        if len(pixels) < 32:
            continue
        spread = _brightness_spread(pixels)
        complexity_bucket = 0 if spread <= 18 else 1
        area = (candidate[2] - candidate[0]) * (candidate[3] - candidate[1])
        score = (complexity_bucket, spread, -area)
        if best_score is None or score < best_score:
            best_score = score
            best_patch = patch
    if best_score is None or best_score[0] > 0 or best_patch is None:
        return None
    x0, y0, x1, y1 = box
    return best_patch.resize((x1 - x0, y1 - y0))


def _sample_background_color(image: Image.Image, box: tuple[int, int, int, int]) -> tuple[int, int, int]:
    width, height = image.size
    x0, y0, x1, y1 = box
    margin = max(8, min(28, int(min(x1 - x0, y1 - y0) * 0.4)))
    outer = (
        max(0, x0 - margin),
        max(0, y0 - margin),
        min(width, x1 + margin),
        min(height, y1 + margin),
    )
    sample = image.crop(outer)
    inner_x0 = x0 - outer[0]
    inner_y0 = y0 - outer[1]
    inner_x1 = x1 - outer[0]
    inner_y1 = y1 - outer[1]
    pixels: list[tuple[int, int, int]] = []
    for yy in range(sample.height):
        inside_y = inner_y0 <= yy < inner_y1
        for xx in range(sample.width):
            if inside_y and inner_x0 <= xx < inner_x1:
                continue
            pixels.append(sample.getpixel((xx, yy)))
    if not pixels:
        return (255, 255, 255)
    rs = sorted(pixel[0] for pixel in pixels)
    gs = sorted(pixel[1] for pixel in pixels)
    bs = sorted(pixel[2] for pixel in pixels)
    return (
        quantile(rs, 1, 2),
        quantile(gs, 1, 2),
        quantile(bs, 1, 2),
    )


def _rewrite_background_image(
    image: Image.Image,
    image_rect: fitz.Rect,
    rects: list[fitz.Rect],
) -> Image.Image:
    updated = image.copy()
    for rect in rects:
        mapped = _map_rect_to_image(image_rect, updated.size, rect)
        if mapped is None:
            continue
        patch = _pick_background_patch(updated, mapped)
        if patch is not None:
            updated.paste(patch, (mapped[0], mapped[1]))
            continue
        updated.paste(_sample_background_color(updated, mapped), mapped)
    return updated


def _rebuilt_image_bytes(image: Image.Image, payload: dict | None) -> bytes:
    ext = str((payload or {}).get("ext", "") or "").lower()
    buffer = BytesIO()
    if ext in {"jpg", "jpeg"}:
        image.convert("RGB").save(buffer, format="JPEG", quality=85, optimize=True)
        return buffer.getvalue()

    if image.mode not in {"RGB", "RGBA", "L", "LA", "1", "P"}:
        image = image.convert("RGB")
    image.save(buffer, format="PNG", optimize=True)
    return buffer.getvalue()


def _rewrite_raw_stream_image(
    image: Image.Image,
    image_rect: fitz.Rect,
    rects: list[fitz.Rect],
    *,
    fill,
) -> Image.Image:
    updated = image.copy()
    draw = ImageDraw.Draw(updated)
    for rect in rects:
        mapped = _map_rect_to_image(image_rect, updated.size, rect)
        if mapped is None:
            continue
        x0, y0, x1, y1 = mapped
        draw.rectangle((x0, y0, max(x0, x1 - 1), max(y0, y1 - 1)), fill=fill)
    return updated


def replace_background_image_page(
    page: fitz.Page,
    translated_items: list[dict],
) -> bool:
    if not page_has_large_background_image(page):
        return False
    primary = _pick_primary_background_image(page)
    if primary is None:
        return False

    xref, image_rect = primary
    doc = page.parent
    rects: list[fitz.Rect] = []
    for _rect, item, _translated_text in iter_valid_translated_items(translated_items):
        bbox = item.get("bbox", [])
        if len(bbox) != 4:
            continue
        rects.append(fitz.Rect(bbox))
    rects = _merge_close_vertical_rects(rects)
    if not rects:
        return False

    raw_meta = _raw_stream_image_meta(doc, xref)
    if raw_meta is not None:
        raw_image = _extract_raw_stream_image(doc, xref, raw_meta)
        if raw_image is not None:
            rebuilt_raw = _rewrite_raw_stream_image(raw_image, image_rect, rects, fill=raw_meta["fill"])
            try:
                doc.update_stream(xref, rebuilt_raw.tobytes(), new=0, compress=1)
                return True
            except Exception:
                pass

    payload = _extract_image_payload(doc, xref)
    image = _extract_image_rgb(doc, xref)
    if image is None:
        return False

    rebuilt = _rewrite_background_image(image, image_rect, rects)
    image_bytes = _rebuilt_image_bytes(rebuilt, payload)
    try:
        page.replace_image(xref, stream=image_bytes)
        return True
    except Exception:
        return False


__all__ = ["replace_background_image_page"]
