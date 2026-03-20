# `std2_manual.json` 结构说明

这个文件是 `std2_manual.pdf` 的 OCR 结果，按“文档 -> 页面 -> 段落块 -> 行 -> span”分层组织。

## 文件位置

- PDF 原文件: `Data/std2_manual.pdf`
- OCR JSON: `Data/std2_manual.json`

## 顶层结构

顶层是一个 `dict`，主要字段如下：

```json
{
  "pdf_info": [...],
  "_backend": "...",
  "_ocr_enable": true,
  "_vlm_ocr_enable": false,
  "_version_name": "..."
}
```

字段含义：

- `pdf_info`: 页面级 OCR 结果列表
- `_backend`: OCR 后端名称
- `_ocr_enable`: 是否启用 OCR
- `_vlm_ocr_enable`: 是否启用 VLM OCR
- `_version_name`: 结果版本信息

## 页面层

`pdf_info` 是一个列表；当前文件中共有 `21` 页。每一页是一个对象，结构类似：

```json
{
  "para_blocks": [...],
  "discarded_blocks": [...],
  "page_size": [width, height],
  "page_idx": 0
}
```

字段含义：

- `para_blocks`: 被保留的段落块
- `discarded_blocks`: 被丢弃的块
- `page_size`: 页面尺寸
- `page_idx`: 页码索引，从 `0` 开始

## 段落块层

`para_blocks` 是段落块列表；当前文件总计约 `245` 个块。常见块结构如下：

```json
{
  "bbox": [x0, y0, x1, y1],
  "type": "title",
  "angle": 0,
  "index": 0,
  "lines": [...]
}
```

常见字段：

- `bbox`: 边界框坐标
- `type`: 块类型，例如 `title`、正文相关类型等
- `angle`: 旋转角度
- `index`: 块内索引
- `lines`: 行列表

少数块还会出现：

- `blocks`
- `sub_type`

## 行层

`lines` 是行列表，每一项通常类似：

```json
{
  "bbox": [x0, y0, x1, y1],
  "spans": [...]
}
```

字段含义：

- `bbox`: 行的边界框
- `spans`: 该行内的文本片段列表

## Span 层

`spans` 是最接近 OCR 文本内容的一层，结构类似：

```json
{
  "bbox": [x0, y0, x1, y1],
  "type": "text",
  "content": "s t d 2",
  "score": 1.0
}
```

字段含义：

- `bbox`: 文本片段边界框
- `type`: 片段类型，当前抽样中为 `text`
- `content`: OCR 识别出的文本
- `score`: OCR 置信度

## 推荐读取路径

如果目标是提取正文内容，推荐按下面的路径遍历：

```text
pdf_info
  -> page
  -> para_blocks
  -> lines
  -> spans
  -> content
```

简化伪代码：

```python
for page in data["pdf_info"]:
    for block in page.get("para_blocks", []):
        for line in block.get("lines", []):
            for span in line.get("spans", []):
                text = span.get("content", "")
```

## 当前文件抽样结论

- 顶层类型是 `dict`
- 页面数是 `21`
- `pdf_info[*]` 的核心键稳定
- `para_blocks[*]` 以 `bbox/type/angle/index/lines` 为主
- 实际文本主要位于 `spans[*].content`

## 适合后续做的事

- 提取整本文本
- 按页导出文本
- 保留坐标信息做版面还原
- 按 `type` 过滤标题、正文、图表说明等内容
