function numberOrNull(value) {
  const num = Number(value);
  return Number.isFinite(num) ? num : null;
}

function firstNonEmpty(...values) {
  for (const value of values) {
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  return "";
}

function stageKeyOf(payload) {
  return firstNonEmpty(payload.current_stage, payload.stage, payload.runtime?.current_stage).toLowerCase();
}

function stageCountsText(payload, stageKey = stageKeyOf(payload)) {
  const current = numberOrNull(payload.progress_current ?? payload.progress?.current);
  const total = numberOrNull(payload.progress_total ?? payload.progress?.total);
  if (current === null || total === null || total <= 0) {
    return "";
  }
  if (stageKey.includes("translat")) {
    return `已完成第 ${current}/${total} 批翻译`;
  }
  if (stageKey.includes("ocr") || stageKey.includes("parse")) {
    return `已完成第 ${current}/${total} 页 OCR`;
  }
  if (stageKey.includes("render")) {
    return `已完成第 ${current}/${total} 页渲染`;
  }
  return `进度 ${current}/${total}`;
}

export function summarizeStageLabel(payload) {
  const stageKey = stageKeyOf(payload);
  if (payload.status === "succeeded") {
    return "处理完成";
  }
  if (payload.status === "failed") {
    return "处理失败";
  }
  if (payload.status === "canceled") {
    return "任务已取消";
  }
  if (stageKey.includes("queue")) {
    return "排队中";
  }
  if (stageKey.includes("ocr") || stageKey.includes("parse")) {
    return "OCR 中";
  }
  if (stageKey.includes("translat")) {
    return "翻译中";
  }
  if (stageKey.includes("normaliz")) {
    return "标准化中";
  }
  if (stageKey.includes("render")) {
    return "渲染中";
  }
  if (stageKey.includes("sav")) {
    return "保存中";
  }
  if (stageKey.includes("finish")) {
    return payload.status === "running" ? "处理中" : "处理完成";
  }
  if (payload.status === "queued") {
    return "排队中";
  }
  if (payload.status === "running") {
    return "处理中";
  }
  return "等待中";
}

export function summarizeStageDetail(payload) {
  const detail = firstNonEmpty(
    payload.failure?.summary,
    payload.stage_detail,
  );
  const stageLabel = summarizeStageLabel(payload);
  const countsText = stageCountsText(payload);
  if (detail) {
    if (detail === stageLabel) {
      return countsText || detail;
    }
    return countsText && !detail.includes(`${payload.progress_current ?? ""}/${payload.progress_total ?? ""}`)
      ? `${detail} · ${countsText}`
      : detail;
  }
  if (countsText) {
    return countsText;
  }
  const currentStage = firstNonEmpty(
    payload.runtime?.current_stage,
    payload.current_stage,
  );
  if (currentStage && currentStage !== stageLabel) {
    return `${stageLabel} · ${currentStage}`;
  }
  return stageLabel || "-";
}
