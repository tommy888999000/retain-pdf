use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

pub const LOG_TAIL_LIMIT: usize = 40;

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn build_job_id() -> String {
    let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let rand = format!("{:06x}", fastrand::u32(..=0xFFFFFF));
    format!("{ts}-{rand}")
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            message: "ok".to_string(),
            data,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum JobStatusKind {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowKind {
    Mineru,
}

impl Default for WorkflowKind {
    fn default() -> Self {
        Self::Mineru
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadRecord {
    pub upload_id: String,
    pub filename: String,
    pub stored_path: String,
    pub bytes: u64,
    pub page_count: u32,
    pub uploaded_at: String,
    pub developer_mode: bool,
}

#[derive(Debug, Serialize)]
pub struct UploadResponseData {
    pub upload_id: String,
    pub filename: String,
    pub bytes: u64,
    pub page_count: u32,
    pub uploaded_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateJobRequest {
    #[serde(default)]
    pub workflow: WorkflowKind,
    pub upload_id: String,
    #[serde(default)]
    pub job_id: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub skip_title_translation: bool,
    #[serde(default = "default_classify_batch_size")]
    pub classify_batch_size: i64,
    #[serde(default = "default_rule_profile_name")]
    pub rule_profile_name: String,
    #[serde(default)]
    pub custom_rules_text: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default = "default_render_mode")]
    pub render_mode: String,
    #[serde(default)]
    pub compile_workers: i64,
    #[serde(default = "default_typst_font_family")]
    pub typst_font_family: String,
    #[serde(default = "default_pdf_compress_dpi")]
    pub pdf_compress_dpi: i64,
    #[serde(default)]
    pub start_page: i64,
    #[serde(default = "default_end_page")]
    pub end_page: i64,
    #[serde(default = "default_batch_size")]
    pub batch_size: i64,
    #[serde(default)]
    pub workers: i64,
    #[serde(default = "default_output_root")]
    pub output_root: String,
    #[serde(default)]
    pub translated_pdf_name: String,
    #[serde(default)]
    pub mineru_token: String,
    #[serde(default = "default_model_version")]
    pub model_version: String,
    #[serde(default)]
    pub is_ocr: bool,
    #[serde(default)]
    pub disable_formula: bool,
    #[serde(default)]
    pub disable_table: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub page_ranges: String,
    #[serde(default)]
    pub data_id: String,
    #[serde(default)]
    pub no_cache: bool,
    #[serde(default = "default_cache_tolerance")]
    pub cache_tolerance: i64,
    #[serde(default)]
    pub extra_formats: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval: i64,
    #[serde(default = "default_poll_timeout")]
    pub poll_timeout: i64,
    #[serde(default = "default_body_font_size_factor")]
    pub body_font_size_factor: f64,
    #[serde(default = "default_body_leading_factor")]
    pub body_leading_factor: f64,
    #[serde(default = "default_inner_bbox_shrink_x")]
    pub inner_bbox_shrink_x: f64,
    #[serde(default = "default_inner_bbox_shrink_y")]
    pub inner_bbox_shrink_y: f64,
    #[serde(default = "default_inner_bbox_dense_shrink_x")]
    pub inner_bbox_dense_shrink_x: f64,
    #[serde(default = "default_inner_bbox_dense_shrink_y")]
    pub inner_bbox_dense_shrink_y: f64,
}

impl CreateJobRequest {
    pub fn resolved_job_id(&self) -> String {
        if self.job_id.trim().is_empty() {
            build_job_id()
        } else {
            self.job_id.trim().to_string()
        }
    }

    pub fn resolved_workers(&self) -> i64 {
        if self.workers > 0 {
            return self.workers;
        }
        let model = self.model.to_lowercase();
        let base = self.base_url.to_lowercase();
        if model.contains("deepseek") || base.contains("deepseek.com") {
            100
        } else {
            4
        }
    }
}

impl Default for CreateJobRequest {
    fn default() -> Self {
        Self {
            workflow: WorkflowKind::default(),
            upload_id: String::new(),
            job_id: String::new(),
            mode: default_mode(),
            skip_title_translation: false,
            classify_batch_size: default_classify_batch_size(),
            rule_profile_name: default_rule_profile_name(),
            custom_rules_text: String::new(),
            api_key: String::new(),
            model: String::new(),
            base_url: String::new(),
            render_mode: default_render_mode(),
            compile_workers: 0,
            typst_font_family: default_typst_font_family(),
            pdf_compress_dpi: default_pdf_compress_dpi(),
            start_page: 0,
            end_page: default_end_page(),
            batch_size: default_batch_size(),
            workers: 0,
            output_root: default_output_root(),
            translated_pdf_name: String::new(),
            mineru_token: String::new(),
            model_version: default_model_version(),
            is_ocr: false,
            disable_formula: false,
            disable_table: false,
            language: default_language(),
            page_ranges: String::new(),
            data_id: String::new(),
            no_cache: false,
            cache_tolerance: default_cache_tolerance(),
            extra_formats: String::new(),
            poll_interval: default_poll_interval(),
            poll_timeout: default_poll_timeout(),
            body_font_size_factor: default_body_font_size_factor(),
            body_leading_factor: default_body_leading_factor(),
            inner_bbox_shrink_x: default_inner_bbox_shrink_x(),
            inner_bbox_shrink_y: default_inner_bbox_shrink_y(),
            inner_bbox_dense_shrink_x: default_inner_bbox_dense_shrink_x(),
            inner_bbox_dense_shrink_y: default_inner_bbox_dense_shrink_y(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateJobResponseData {
    pub job_id: String,
    pub status: JobStatusKind,
    pub workflow: WorkflowKind,
    pub links: JobLinksData,
    pub actions: JobActionsData,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct JobArtifacts {
    pub job_root: Option<String>,
    pub source_pdf: Option<String>,
    pub layout_json: Option<String>,
    pub translations_dir: Option<String>,
    pub output_pdf: Option<String>,
    pub summary: Option<String>,
    pub pages_processed: Option<i64>,
    pub translated_items: Option<i64>,
    pub translate_render_time_seconds: Option<f64>,
    pub save_time_seconds: Option<f64>,
    pub total_time_seconds: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProcessResult {
    pub success: bool,
    pub return_code: i32,
    pub duration_seconds: f64,
    pub command: Vec<String>,
    pub cwd: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredJob {
    pub job_id: String,
    pub workflow: WorkflowKind,
    pub status: JobStatusKind,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub upload_id: Option<String>,
    pub pid: Option<u32>,
    pub command: Vec<String>,
    pub request_payload: CreateJobRequest,
    pub error: Option<String>,
    pub stage: Option<String>,
    pub stage_detail: Option<String>,
    pub progress_current: Option<i64>,
    pub progress_total: Option<i64>,
    pub log_tail: Vec<String>,
    pub result: Option<ProcessResult>,
    pub artifacts: Option<JobArtifacts>,
}

impl StoredJob {
    pub fn new(job_id: String, request_payload: CreateJobRequest, command: Vec<String>) -> Self {
        let now = now_iso();
        Self {
            job_id,
            workflow: request_payload.workflow.clone(),
            status: JobStatusKind::Queued,
            created_at: now.clone(),
            updated_at: now,
            started_at: None,
            finished_at: None,
            upload_id: Some(request_payload.upload_id.clone()),
            pid: None,
            command,
            request_payload,
            error: None,
            stage: Some("queued".to_string()),
            stage_detail: Some("任务已创建，等待可用执行槽位".to_string()),
            progress_current: Some(0),
            progress_total: None,
            log_tail: Vec::new(),
            result: None,
            artifacts: Some(JobArtifacts::default()),
        }
    }

    pub fn append_log(&mut self, line: &str) {
        let text = line.trim();
        if text.is_empty() {
            return;
        }
        self.log_tail.push(text.to_string());
        if self.log_tail.len() > LOG_TAIL_LIMIT {
            let drain = self.log_tail.len() - LOG_TAIL_LIMIT;
            self.log_tail.drain(0..drain);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JobProgressData {
    pub current: Option<i64>,
    pub total: Option<i64>,
    pub percent: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct JobTimestampsData {
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct JobLinksData {
    pub self_path: String,
    pub self_url: String,
    pub artifacts_path: String,
    pub artifacts_url: String,
    pub cancel_path: String,
    pub cancel_url: String,
}

#[derive(Debug, Serialize)]
pub struct ActionLinkData {
    pub enabled: bool,
    pub method: String,
    pub path: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct JobActionsData {
    pub open_job: ActionLinkData,
    pub open_artifacts: ActionLinkData,
    pub cancel: ActionLinkData,
    pub download_pdf: ActionLinkData,
    pub open_markdown: ActionLinkData,
    pub open_markdown_raw: ActionLinkData,
    pub download_bundle: ActionLinkData,
}

#[derive(Debug, Serialize)]
pub struct ResourceLinkData {
    pub ready: bool,
    pub path: String,
    pub url: String,
    pub method: String,
    pub content_type: String,
    pub file_name: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct MarkdownArtifactData {
    pub ready: bool,
    pub json_path: String,
    pub json_url: String,
    pub raw_path: String,
    pub raw_url: String,
    pub images_base_path: String,
    pub images_base_url: String,
    pub file_name: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ArtifactLinksData {
    pub pdf_ready: bool,
    pub markdown_ready: bool,
    pub bundle_ready: bool,
    pub pdf_url: String,
    pub markdown_url: String,
    pub markdown_images_base_url: String,
    pub bundle_url: String,
    pub actions: JobActionsData,
    pub pdf: ResourceLinkData,
    pub markdown: MarkdownArtifactData,
    pub bundle: ResourceLinkData,
}

#[derive(Debug, Serialize)]
pub struct JobDetailData {
    pub job_id: String,
    pub workflow: WorkflowKind,
    pub status: JobStatusKind,
    pub stage: Option<String>,
    pub stage_detail: Option<String>,
    pub progress: JobProgressData,
    pub timestamps: JobTimestampsData,
    pub links: JobLinksData,
    pub actions: JobActionsData,
    pub artifacts: ArtifactLinksData,
    pub log_tail: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct JobListItemData {
    pub job_id: String,
    pub workflow: WorkflowKind,
    pub status: JobStatusKind,
    pub stage: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub detail_path: String,
    pub detail_url: String,
}

#[derive(Debug, Serialize)]
pub struct JobListResponseData {
    pub items: Vec<JobListItemData>,
}

#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[derive(Debug, Deserialize)]
pub struct MarkdownQuery {
    #[serde(default)]
    pub raw: bool,
}

#[derive(Debug, Serialize)]
pub struct MarkdownResponseData {
    pub job_id: String,
    pub content: String,
    pub raw_path: String,
    pub raw_url: String,
    pub images_base_path: String,
    pub images_base_url: String,
}

pub fn build_job_links(job_id: &str, base_url: &str) -> JobLinksData {
    let self_path = format!("/api/v1/jobs/{job_id}");
    let artifacts_path = format!("/api/v1/jobs/{job_id}/artifacts");
    let cancel_path = format!("/api/v1/jobs/{job_id}/cancel");
    JobLinksData {
        self_path: self_path.clone(),
        self_url: to_absolute_url(base_url, &self_path),
        artifacts_path: artifacts_path.clone(),
        artifacts_url: to_absolute_url(base_url, &artifacts_path),
        cancel_path: cancel_path.clone(),
        cancel_url: to_absolute_url(base_url, &cancel_path),
    }
}

fn can_cancel(status: &JobStatusKind) -> bool {
    matches!(status, JobStatusKind::Queued | JobStatusKind::Running)
}

fn action_link(enabled: bool, method: &str, path: String, base_url: &str) -> ActionLinkData {
    ActionLinkData {
        enabled,
        method: method.to_string(),
        url: to_absolute_url(base_url, &path),
        path,
    }
}

pub fn build_job_actions(
    job: &StoredJob,
    base_url: &str,
    pdf_ready: bool,
    markdown_ready: bool,
    bundle_ready: bool,
) -> JobActionsData {
    let job_path = format!("/api/v1/jobs/{}", job.job_id);
    let artifacts_path = format!("/api/v1/jobs/{}/artifacts", job.job_id);
    let cancel_path = format!("/api/v1/jobs/{}/cancel", job.job_id);
    let pdf_path = format!("/api/v1/jobs/{}/pdf", job.job_id);
    let markdown_path = format!("/api/v1/jobs/{}/markdown", job.job_id);
    let markdown_raw_path = format!("/api/v1/jobs/{}/markdown?raw=true", job.job_id);
    let bundle_path = format!("/api/v1/jobs/{}/download", job.job_id);
    JobActionsData {
        open_job: action_link(true, "GET", job_path, base_url),
        open_artifacts: action_link(true, "GET", artifacts_path, base_url),
        cancel: action_link(can_cancel(&job.status), "POST", cancel_path, base_url),
        download_pdf: action_link(pdf_ready, "GET", pdf_path, base_url),
        open_markdown: action_link(markdown_ready, "GET", markdown_path, base_url),
        open_markdown_raw: action_link(markdown_ready, "GET", markdown_raw_path, base_url),
        download_bundle: action_link(bundle_ready, "GET", bundle_path, base_url),
    }
}

pub fn build_artifact_links(
    job: &StoredJob,
    base_url: &str,
    pdf_ready: bool,
    markdown_ready: bool,
    bundle_ready: bool,
) -> ArtifactLinksData {
    let pdf_path = format!("/api/v1/jobs/{}/pdf", job.job_id);
    let markdown_path = format!("/api/v1/jobs/{}/markdown", job.job_id);
    let markdown_raw_path = format!("/api/v1/jobs/{}/markdown?raw=true", job.job_id);
    let markdown_images_base_path = format!("/api/v1/jobs/{}/markdown/images/", job.job_id);
    let bundle_path = format!("/api/v1/jobs/{}/download", job.job_id);
    let pdf_file_path = resolve_output_pdf(job);
    let markdown_file_path = resolve_markdown_path(job);
    let bundle_file_name = format!("{}.zip", job.job_id);
    let actions = build_job_actions(job, base_url, pdf_ready, markdown_ready, bundle_ready);
    ArtifactLinksData {
        pdf_ready,
        markdown_ready,
        bundle_ready,
        pdf_url: pdf_path.clone(),
        markdown_url: markdown_path.clone(),
        markdown_images_base_url: markdown_images_base_path.clone(),
        bundle_url: bundle_path.clone(),
        actions,
        pdf: ResourceLinkData {
            ready: pdf_ready,
            path: pdf_path.clone(),
            url: to_absolute_url(base_url, &pdf_path),
            method: "GET".to_string(),
            content_type: "application/pdf".to_string(),
            file_name: file_name_from_path(pdf_file_path.as_deref()),
            size_bytes: file_size(pdf_file_path.as_deref()),
        },
        markdown: MarkdownArtifactData {
            ready: markdown_ready,
            json_path: markdown_path.clone(),
            json_url: to_absolute_url(base_url, &markdown_path),
            raw_path: markdown_raw_path.clone(),
            raw_url: to_absolute_url(base_url, &markdown_raw_path),
            images_base_path: markdown_images_base_path.clone(),
            images_base_url: to_absolute_url(base_url, &markdown_images_base_path),
            file_name: file_name_from_path(markdown_file_path.as_deref()),
            size_bytes: file_size(markdown_file_path.as_deref()),
        },
        bundle: ResourceLinkData {
            ready: bundle_ready,
            path: bundle_path.clone(),
            url: to_absolute_url(base_url, &bundle_path),
            method: "GET".to_string(),
            content_type: "application/zip".to_string(),
            file_name: Some(bundle_file_name),
            size_bytes: None,
        },
    }
}

pub fn job_to_detail(
    job: &StoredJob,
    base_url: &str,
    pdf_ready: bool,
    markdown_ready: bool,
    bundle_ready: bool,
) -> JobDetailData {
    let duration_seconds = match (&job.started_at, &job.finished_at, &job.result) {
        (_, _, Some(result)) => Some(result.duration_seconds),
        _ => None,
    };
    let percent = match (job.progress_current, job.progress_total) {
        (Some(current), Some(total)) if total > 0 => Some((current as f64 / total as f64) * 100.0),
        _ => None,
    };
    JobDetailData {
        job_id: job.job_id.clone(),
        workflow: job.workflow.clone(),
        status: job.status.clone(),
        stage: job.stage.clone(),
        stage_detail: job.stage_detail.clone(),
        progress: JobProgressData {
            current: job.progress_current,
            total: job.progress_total,
            percent,
        },
        timestamps: JobTimestampsData {
            created_at: job.created_at.clone(),
            updated_at: job.updated_at.clone(),
            started_at: job.started_at.clone(),
            finished_at: job.finished_at.clone(),
            duration_seconds,
        },
        links: build_job_links(&job.job_id, base_url),
        actions: build_job_actions(job, base_url, pdf_ready, markdown_ready, bundle_ready),
        artifacts: build_artifact_links(job, base_url, pdf_ready, markdown_ready, bundle_ready),
        log_tail: job.log_tail.clone(),
    }
}

pub fn job_to_list_item(job: &StoredJob, base_url: &str) -> JobListItemData {
    let detail_path = format!("/api/v1/jobs/{}", job.job_id);
    JobListItemData {
        job_id: job.job_id.clone(),
        workflow: job.workflow.clone(),
        status: job.status.clone(),
        stage: job.stage.clone(),
        created_at: job.created_at.clone(),
        updated_at: job.updated_at.clone(),
        detail_url: to_absolute_url(base_url, &detail_path),
        detail_path,
    }
}

pub fn upload_to_response(upload: &UploadRecord) -> UploadResponseData {
    UploadResponseData {
        upload_id: upload.upload_id.clone(),
        filename: upload.filename.clone(),
        bytes: upload.bytes,
        page_count: upload.page_count,
        uploaded_at: upload.uploaded_at.clone(),
    }
}

pub fn resolve_markdown_path(job: &StoredJob) -> Option<PathBuf> {
    let job_root = job.artifacts.as_ref()?.job_root.as_ref()?;
    Some(resolve_project_path(job_root).join("ocr").join("unpacked").join("full.md"))
}

pub fn resolve_markdown_images_dir(job: &StoredJob) -> Option<PathBuf> {
    let job_root = job.artifacts.as_ref()?.job_root.as_ref()?;
    Some(resolve_project_path(job_root).join("ocr").join("unpacked").join("images"))
}

pub fn resolve_output_pdf(job: &StoredJob) -> Option<PathBuf> {
    let path = job.artifacts.as_ref()?.output_pdf.as_ref()?;
    Some(resolve_project_path(path))
}

fn file_name_from_path(path: Option<&Path>) -> Option<String> {
    path.and_then(|p| p.file_name()).map(|v| v.to_string_lossy().to_string())
}

fn file_size(path: Option<&Path>) -> Option<u64> {
    path.and_then(|p| std::fs::metadata(p).ok()).map(|meta| meta.len())
}

fn resolve_project_path(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return path;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(path)
}

pub fn to_absolute_url(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

fn default_mode() -> String {
    "sci".to_string()
}
fn default_classify_batch_size() -> i64 {
    12
}
fn default_rule_profile_name() -> String {
    "general_sci".to_string()
}
fn default_render_mode() -> String {
    "auto".to_string()
}
fn default_typst_font_family() -> String {
    "Source Han Serif SC".to_string()
}
fn default_pdf_compress_dpi() -> i64 {
    200
}
fn default_end_page() -> i64 {
    -1
}
fn default_batch_size() -> i64 {
    1
}
fn default_output_root() -> String {
    "output".to_string()
}
fn default_model_version() -> String {
    "vlm".to_string()
}
fn default_language() -> String {
    "ch".to_string()
}
fn default_cache_tolerance() -> i64 {
    900
}
fn default_poll_interval() -> i64 {
    5
}
fn default_poll_timeout() -> i64 {
    1800
}
fn default_body_font_size_factor() -> f64 {
    0.95
}
fn default_body_leading_factor() -> f64 {
    1.08
}
fn default_inner_bbox_shrink_x() -> f64 {
    0.035
}
fn default_inner_bbox_shrink_y() -> f64 {
    0.04
}
fn default_inner_bbox_dense_shrink_x() -> f64 {
    0.025
}
fn default_inner_bbox_dense_shrink_y() -> f64 {
    0.03
}
fn default_limit() -> u32 {
    20
}
