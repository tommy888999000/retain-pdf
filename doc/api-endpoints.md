# 接口说明

## 1. 上传 PDF

`POST /api/v1/uploads`

表单字段：

- `file`：必填，PDF 文件

示例：

```bash
curl -X POST http://127.0.0.1:41000/api/v1/uploads \
  -H "X-API-Key: your-rust-api-key" \
  -F "file=@/path/to/paper.pdf"
```

## 2. 创建主任务

`POST /api/v1/jobs`

当前正式 JSON 契约是 grouped request body，不再接受旧扁平字段。

最常用请求体：

```json
{
  "workflow": "book",
  "source": {
    "upload_id": "20260402073151-a80618"
  },
  "ocr": {
    "provider": "mineru",
    "mineru_token": "mineru-xxxx",
    "model_version": "vlm",
    "language": "ch",
    "page_ranges": ""
  },
  "translation": {
    "mode": "sci",
    "model": "deepseek-v4-flash",
    "base_url": "https://api.deepseek.com/v1",
    "api_key": "sk-xxxx",
    "skip_title_translation": false,
    "batch_size": 1,
    "workers": 100,
    "classify_batch_size": 12,
    "rule_profile_name": "general_sci",
    "custom_rules_text": ""
  },
  "render": {
    "render_mode": "auto",
    "compile_workers": 8
  },
  "runtime": {
    "timeout_seconds": 1800
  }
}
```

补充说明：

- `workflow=book`：OCR -> Normalize -> Translate -> Render
- `workflow=translate`：OCR -> Normalize -> Translate
- `workflow=render`：基于已有产物重跑渲染，此时 `source.artifact_job_id` 替代 `source.upload_id`
- `workflow=ocr` 使用独立入口 `POST /api/v1/ocr/jobs`，不走这个接口
- `ocr.provider=mineru` 时需要 `ocr.mineru_token`
- 翻译阶段需要 `translation.base_url`、`translation.api_key`、`translation.model`
- `skip_title_translation=false`：翻译标题
- `skip_title_translation=true`：跳过标题翻译，保留原文标题
- 历史扁平字段如 `upload_id`、`mineru_token`、`model`、`render_mode` 不再是 `POST /api/v1/jobs` 的正式 JSON 契约

## 3. 查询任务详情

`GET /api/v1/jobs/{job_id}`

返回重点字段：

- `status`
- `stage`
- `stage_detail`
- `progress`
- `artifacts`
- `ocr_job`
- `failure_diagnostic`
- `log_tail`

## 4. 查询事件流

`GET /api/v1/jobs/{job_id}/events`

用于前端进度展示和排错。

## 5. 下载产物

- `GET /api/v1/jobs/{job_id}/pdf`
- `GET /api/v1/jobs/{job_id}/markdown`
- `GET /api/v1/jobs/{job_id}/markdown?raw=true`
- `GET /api/v1/jobs/{job_id}/download`
- `GET /api/v1/jobs/{job_id}/normalized-document`
- `GET /api/v1/jobs/{job_id}/normalization-report`

## 6. 取消任务

`POST /api/v1/jobs/{job_id}/cancel`

## 7. OCR 凭证检测

- `POST /api/v1/providers/mineru/validate-token`
- `POST /api/v1/providers/paddle/validate-token`

示例：

```json
{
  "paddle_token": "paddle-access-token",
  "base_url": "https://paddleocr.aistudio-app.com"
}
```

返回重点字段：

- `ok`
- `status`
- `summary`
- `retryable`
- `provider_code`
- `provider_message`
- `operator_hint`
- `trace_id`

补充说明：

- Paddle 检测不会提交真实 OCR 任务，而是用随机 `jobId` 做只鉴权探测
- 当 Paddle 返回“任务不存在 / 404”时，后端会视为鉴权通过
- 401 / 403 仍会被视为 token 无效

## 8. 常见状态

`status`：

- `queued`
- `running`
- `succeeded`
- `failed`
- `canceled`

常见 `stage`：

- `queued`
- `ocr_submitting`
- `ocr_upload`
- `mineru_upload`
- `mineru_processing`
- `translation_prepare`
- `normalizing`
- `domain_inference`
- `continuation_review`
- `page_policies`
- `translating`
- `rendering`
- `saving`
- `finished`
- `failed`
