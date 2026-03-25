# MinerU Integration

Recommended entry points:

- `../../run_mineru_case.py`
  Recommended top-level one-command entry. Use this first for normal parse -> translate -> render runs.
- `mineru_pipeline.py`
  Stable implementation behind `scripts/run_mineru_case.py`. Parse with MinerU, then translate from `layout.json`, then render into `translated`.
- `mineru_job.py`
  Parse only. Download and unpack MinerU outputs into a structured job directory.
- `migrate_legacy_output.py`
  Move old `output/mineru/<case>` experiments into the new structured job layout.
- `mineru_api.py`
  Low-level API caller. Use this only when you want raw MinerU API interaction.

Structured job layout:

- `output/<job-id>/source`
- `output/<job-id>/ocr`
- `output/<job-id>/translated`
- `output/<job-id>/typst`

Main rule:

- use `ocr/unpacked/layout.json` as the default MinerU OCR JSON for the translation pipeline
- keep `content_list_v2.json` for experiments only

Legacy note:

- old jobs under `originPDF/jsonPDF/transPDF` are still supported for reading and download
