from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import fitz


VECTOR_SKIP_PAGE_DRAWINGS_THRESHOLD = 100
VECTOR_SKIP_TOTAL_DRAWINGS_THRESHOLD = 300


def _page_drawing_count(page: fitz.Page) -> int:
    if hasattr(page, "get_cdrawings"):
        try:
            return len(page.get_cdrawings())
        except Exception:
            pass
    return len(page.get_drawings())


def source_pdf_has_vector_graphics(
    source_pdf_path: Path,
    *,
    start_page: int = 0,
    end_page: int = -1,
) -> bool:
    if not source_pdf_path.exists():
        return False

    doc = fitz.open(source_pdf_path)
    try:
        if len(doc) == 0:
            return False
        start = max(0, start_page)
        stop = len(doc) - 1 if end_page < 0 else min(end_page, len(doc) - 1)
        if start > stop:
            return False

        total_drawings = 0
        for page_idx in range(start, stop + 1):
            drawings = _page_drawing_count(doc[page_idx])
            total_drawings += drawings
            if drawings >= VECTOR_SKIP_PAGE_DRAWINGS_THRESHOLD:
                return True
            if total_drawings >= VECTOR_SKIP_TOTAL_DRAWINGS_THRESHOLD:
                return True
        return False
    finally:
        doc.close()


def compress_pdf_with_ghostscript(
    pdf_path: Path,
    *,
    dpi: int = 200,
    source_pdf_path: Path | None = None,
    render_mode: str | None = None,
    start_page: int = 0,
    end_page: int = -1,
) -> bool:
    if dpi <= 0:
        return False
    gs_bin = shutil.which("gs")
    if not gs_bin:
        return False
    if not pdf_path.exists():
        return False
    if render_mode == "overlay" and source_pdf_path and source_pdf_has_vector_graphics(
        source_pdf_path,
        start_page=start_page,
        end_page=end_page,
    ):
        print(
            "skip Ghostscript: vector-heavy source PDF detected for overlay mode",
            flush=True,
        )
        return False

    temp_path = pdf_path.with_name(f"{pdf_path.stem}.tmp-compressed.pdf")
    command = [
        gs_bin,
        "-sDEVICE=pdfwrite",
        "-dCompatibilityLevel=1.6",
        "-dNOPAUSE",
        "-dQUIET",
        "-dBATCH",
        "-dDetectDuplicateImages=true",
        "-dCompressFonts=true",
        "-dDownsampleColorImages=true",
        f"-dColorImageResolution={dpi}",
        "-dDownsampleGrayImages=true",
        f"-dGrayImageResolution={dpi}",
        "-dDownsampleMonoImages=false",
        f"-sOutputFile={temp_path}",
        str(pdf_path),
    ]
    try:
        proc = subprocess.run(command, capture_output=True, text=True)
        if proc.returncode != 0 or not temp_path.exists():
            if temp_path.exists():
                temp_path.unlink(missing_ok=True)
            return False
        temp_path.replace(pdf_path)
        return True
    finally:
        if temp_path.exists():
            temp_path.unlink(missing_ok=True)
