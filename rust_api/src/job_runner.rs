use std::io;
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{OwnedSemaphorePermit, TryAcquireError};
use tokio::time::{sleep, Duration};
use tracing::{error, info};

use crate::models::{now_iso, CreateJobRequest, JobArtifacts, JobStatusKind, ProcessResult, StoredJob};
use crate::AppState;

static JOB_ROOT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^job root:\s*(.+)$").unwrap());
static SOURCE_PDF_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^source pdf:\s*(.+)$").unwrap());
static LAYOUT_JSON_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^layout json:\s*(.+)$").unwrap());
static TRANSLATIONS_DIR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^translations dir:\s*(.+)$").unwrap());
static OUTPUT_PDF_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^output pdf:\s*(.+)$").unwrap());
static SUMMARY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^summary:\s*(.+)$").unwrap());
static PAGES_PROCESSED_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^pages processed:\s*(\d+)$").unwrap());
static TRANSLATED_ITEMS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^translated items:\s*(\d+)$").unwrap());
static TRANSLATE_TIME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^translation time:\s*([0-9.]+)s$").unwrap());
static SAVE_TIME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?:render\+save time|save time):\s*([0-9.]+)s$").unwrap());
static TOTAL_TIME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^total time:\s*([0-9.]+)s$").unwrap());
static MINERU_BATCH_STATE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^batch ([^:]+): state=(.+)$").unwrap());
static PAGE_POLICY_MODE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^book: page policies mode=([a-z_]+) total_pages=(\d+)$").unwrap());
static PAGE_POLICY_PAGE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^book: page policy page (\d+)/(\d+) -> source page (\d+)$").unwrap());
static BATCH_PROGRESS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^book: completed batch (\d+)/(\d+)$").unwrap());
static TRANSLATE_ATTEMPT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^book: batch (\d+)/(\d+): translate attempt").unwrap());
static OVERLAY_MERGE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^overlay merge page (\d+)/(\d+) -> source page (\d+)$").unwrap());
const QUEUE_POLL_INTERVAL_MS: u64 = 250;

pub fn spawn_job(state: AppState, job_id: String) {
    tokio::spawn(async move {
        if let Err(err) = run_job(state.clone(), job_id.clone()).await {
            error!("job {} failed to run: {}", job_id, err);
            if let Ok(mut job) = state.db.get_job(&job_id) {
                if matches!(job.status, JobStatusKind::Canceled) {
                    clear_cancel_request(&state, &job_id).await;
                    return;
                }
                job.status = JobStatusKind::Failed;
                job.stage = Some("failed".to_string());
                job.stage_detail = Some("Rust API worker crashed".to_string());
                job.error = Some(err.to_string());
                job.updated_at = now_iso();
                job.finished_at = Some(now_iso());
                let _ = state.db.save_job(&job);
            }
            clear_cancel_request(&state, &job_id).await;
        }
    });
}

pub fn build_command(state: &AppState, upload_path: &Path, request: &CreateJobRequest, job_id: &str) -> Vec<String> {
    let mut cmd = vec![
        state.config.python_bin.clone(),
        state
            .config
            .run_mineru_case_script
            .to_string_lossy()
            .to_string(),
        "--file-path".to_string(),
        upload_path.to_string_lossy().to_string(),
        "--mineru-token".to_string(),
        request.mineru_token.clone(),
        "--model-version".to_string(),
        request.model_version.clone(),
    ];
    if request.is_ocr {
        cmd.push("--is-ocr".to_string());
    }
    if request.disable_formula {
        cmd.push("--disable-formula".to_string());
    }
    if request.disable_table {
        cmd.push("--disable-table".to_string());
    }
    cmd.extend([
        "--language".to_string(),
        request.language.clone(),
        "--page-ranges".to_string(),
        request.page_ranges.clone(),
        "--data-id".to_string(),
        request.data_id.clone(),
    ]);
    if request.no_cache {
        cmd.push("--no-cache".to_string());
    }
    cmd.extend([
        "--cache-tolerance".to_string(),
        request.cache_tolerance.to_string(),
        "--extra-formats".to_string(),
        request.extra_formats.clone(),
        "--poll-interval".to_string(),
        request.poll_interval.to_string(),
        "--poll-timeout".to_string(),
        request.poll_timeout.to_string(),
        "--job-id".to_string(),
        job_id.to_string(),
        "--output-root".to_string(),
        request.output_root.clone(),
    ]);
    if !request.translated_pdf_name.trim().is_empty() {
        cmd.extend([
            "--translated-pdf-name".to_string(),
            request.translated_pdf_name.trim().to_string(),
        ]);
    }
    cmd.extend([
        "--start-page".to_string(),
        request.start_page.to_string(),
        "--end-page".to_string(),
        request.end_page.to_string(),
        "--batch-size".to_string(),
        request.batch_size.to_string(),
        "--workers".to_string(),
        request.resolved_workers().to_string(),
        "--mode".to_string(),
        request.mode.clone(),
    ]);
    if request.skip_title_translation {
        cmd.push("--skip-title-translation".to_string());
    }
    cmd.extend([
        "--classify-batch-size".to_string(),
        request.classify_batch_size.to_string(),
        "--rule-profile-name".to_string(),
        request.rule_profile_name.clone(),
        "--custom-rules-text".to_string(),
        request.custom_rules_text.clone(),
        "--api-key".to_string(),
        request.api_key.clone(),
        "--model".to_string(),
        request.model.clone(),
        "--base-url".to_string(),
        request.base_url.clone(),
        "--render-mode".to_string(),
        request.render_mode.clone(),
        "--compile-workers".to_string(),
        request.compile_workers.to_string(),
        "--typst-font-family".to_string(),
        request.typst_font_family.clone(),
        "--pdf-compress-dpi".to_string(),
        request.pdf_compress_dpi.to_string(),
        "--body-font-size-factor".to_string(),
        request.body_font_size_factor.to_string(),
        "--body-leading-factor".to_string(),
        request.body_leading_factor.to_string(),
        "--inner-bbox-shrink-x".to_string(),
        request.inner_bbox_shrink_x.to_string(),
        "--inner-bbox-shrink-y".to_string(),
        request.inner_bbox_shrink_y.to_string(),
        "--inner-bbox-dense-shrink-x".to_string(),
        request.inner_bbox_dense_shrink_x.to_string(),
        "--inner-bbox-dense-shrink-y".to_string(),
        request.inner_bbox_dense_shrink_y.to_string(),
    ]);
    cmd
}

async fn run_job(state: AppState, job_id: String) -> Result<()> {
    let mut job = state.db.get_job(&job_id)?;
    if is_cancel_requested(&state, &job_id).await || matches!(job.status, JobStatusKind::Canceled) {
        clear_cancel_request(&state, &job_id).await;
        return Ok(());
    }
    job.status = JobStatusKind::Queued;
    job.stage = Some("queued".to_string());
    job.stage_detail = Some("任务排队中，等待可用执行槽位".to_string());
    job.updated_at = now_iso();
    state.db.save_job(&job)?;

    let _permit = match wait_for_execution_slot(&state, &job_id).await? {
        Some(permit) => permit,
        None => return Ok(()),
    };

    let mut job = state.db.get_job(&job_id)?;
    if is_cancel_requested(&state, &job_id).await || matches!(job.status, JobStatusKind::Canceled) {
        clear_cancel_request(&state, &job_id).await;
        return Ok(());
    }
    let started_at = now_iso();
    job.status = JobStatusKind::Running;
    job.stage = Some("running".to_string());
    job.stage_detail = Some("正在启动 Python worker".to_string());
    job.started_at = Some(started_at);
    job.updated_at = now_iso();

    let mut command = Command::new(&job.command[0]);
    command
        .args(&job.command[1..])
        .current_dir(&state.config.project_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = command.spawn().context("failed to spawn python worker")?;
    job.pid = child.id();
    state.db.save_job(&job)?;
    info!("started job {} pid={:?}", job_id, job.pid);

    if is_cancel_requested(&state, &job_id).await {
        if let Some(pid) = job.pid {
            terminate_job_process_tree(pid).await?;
        }
    }

    let stdout = child.stdout.take().context("missing stdout pipe")?;
    let stderr = child.stderr.take().context("missing stderr pipe")?;

    let stdout_handle = tokio::spawn(read_stdout(state.clone(), job, stdout));
    let stderr_handle = tokio::spawn(read_stream(stderr));
    let started = Instant::now();
    let status = child.wait().await?;
    let stdout_job = stdout_handle.await??;
    let stderr_text = stderr_handle.await??;
    let mut final_job = state.db.get_job(&job_id)?;
    final_job.pid = None;
    final_job.updated_at = now_iso();
    final_job.finished_at = Some(now_iso());
    let duration = started.elapsed().as_secs_f64();

    let stdout_text = stdout_job.0;
    let mut latest_job = stdout_job.1;
    latest_job.updated_at = final_job.updated_at.clone();
    latest_job.finished_at = final_job.finished_at.clone();
    latest_job.pid = None;
    latest_job.result = Some(ProcessResult {
        success: status.success(),
        return_code: status.code().unwrap_or(-1),
        duration_seconds: duration,
        command: latest_job.command.clone(),
        cwd: state.config.project_root.to_string_lossy().to_string(),
        stdout: stdout_text,
        stderr: stderr_text.clone(),
    });

    if matches!(final_job.status, JobStatusKind::Canceled) || is_cancel_requested(&state, &job_id).await {
        latest_job.status = JobStatusKind::Canceled;
        latest_job.stage = Some("canceled".to_string());
        latest_job.stage_detail = Some("任务已取消".to_string());
    } else if status.success() {
        latest_job.status = JobStatusKind::Succeeded;
        latest_job.stage = Some("finished".to_string());
        latest_job.stage_detail = Some("任务完成".to_string());
    } else {
        latest_job.status = JobStatusKind::Failed;
        latest_job.stage = Some("failed".to_string());
        latest_job.stage_detail = Some("Python worker 执行失败".to_string());
        latest_job.error = Some(stderr_text);
    }
    state.db.save_job(&latest_job)?;
    clear_cancel_request(&state, &job_id).await;
    Ok(())
}

async fn wait_for_execution_slot(state: &AppState, job_id: &str) -> Result<Option<OwnedSemaphorePermit>> {
    loop {
        if is_cancel_requested(state, job_id).await {
            clear_cancel_request(state, job_id).await;
            return Ok(None);
        }
        let current_job = state.db.get_job(job_id)?;
        if matches!(current_job.status, JobStatusKind::Canceled) {
            clear_cancel_request(state, job_id).await;
            return Ok(None);
        }
        match state.job_slots.clone().try_acquire_owned() {
            Ok(permit) => return Ok(Some(permit)),
            Err(TryAcquireError::NoPermits) => sleep(Duration::from_millis(QUEUE_POLL_INTERVAL_MS)).await,
            Err(TryAcquireError::Closed) => return Err(anyhow!("job execution slots are closed")),
        }
    }
}

async fn read_stream<R>(reader: R) -> Result<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut out = String::new();
    while let Some(line) = lines.next_line().await? {
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

async fn read_stdout(
    state: AppState,
    mut job: StoredJob,
    stdout: tokio::process::ChildStdout,
) -> Result<(String, StoredJob)> {
    let mut out = String::new();
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines.next_line().await? {
        if is_cancel_requested(&state, &job.job_id).await {
            break;
        }
        out.push_str(&line);
        out.push('\n');
        apply_line(&mut job, &line);
        if is_cancel_requested(&state, &job.job_id).await {
            break;
        }
        job.updated_at = now_iso();
        state.db.save_job(&job)?;
    }
    Ok((out, job))
}

pub async fn request_cancel(state: &AppState, job_id: &str) {
    let mut canceled_jobs = state.canceled_jobs.write().await;
    canceled_jobs.insert(job_id.to_string());
}

pub async fn clear_cancel_request(state: &AppState, job_id: &str) {
    let mut canceled_jobs = state.canceled_jobs.write().await;
    canceled_jobs.remove(job_id);
}

pub async fn is_cancel_requested(state: &AppState, job_id: &str) -> bool {
    let canceled_jobs = state.canceled_jobs.read().await;
    canceled_jobs.contains(job_id)
}

pub async fn terminate_job_process_tree(pid: u32) -> Result<()> {
    let pgid = pid as i32;
    signal_process_group(pgid, libc::SIGTERM)?;
    for _ in 0..15 {
        if !process_group_exists(pgid) {
            return Ok(());
        }
        sleep(Duration::from_millis(200)).await;
    }
    signal_process_group(pgid, libc::SIGKILL)?;
    for _ in 0..10 {
        if !process_group_exists(pgid) {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

fn signal_process_group(pgid: i32, signal: i32) -> Result<()> {
    let rc = unsafe { libc::kill(-pgid, signal) };
    if rc == 0 {
        return Ok(());
    }
    let err = io::Error::last_os_error();
    if matches!(err.raw_os_error(), Some(libc::ESRCH)) {
        return Ok(());
    }
    Err(err.into())
}

fn process_group_exists(pgid: i32) -> bool {
    let rc = unsafe { libc::kill(-pgid, 0) };
    if rc == 0 {
        return true;
    }
    !matches!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH))
}

fn ensure_artifacts(job: &mut StoredJob) -> &mut JobArtifacts {
    if job.artifacts.is_none() {
        job.artifacts = Some(JobArtifacts::default());
    }
    job.artifacts.as_mut().unwrap()
}

fn apply_line(job: &mut StoredJob, line: &str) {
    let stripped = line.trim();
    if stripped.is_empty() {
        return;
    }
    job.append_log(stripped);

    if let Some(caps) = JOB_ROOT_RE.captures(stripped) {
        ensure_artifacts(job).job_root = Some(caps[1].trim().to_string());
    }
    if let Some(caps) = SOURCE_PDF_RE.captures(stripped) {
        ensure_artifacts(job).source_pdf = Some(caps[1].trim().to_string());
    }
    if let Some(caps) = LAYOUT_JSON_RE.captures(stripped) {
        ensure_artifacts(job).layout_json = Some(caps[1].trim().to_string());
    }
    if let Some(caps) = TRANSLATIONS_DIR_RE.captures(stripped) {
        ensure_artifacts(job).translations_dir = Some(caps[1].trim().to_string());
    }
    if let Some(caps) = OUTPUT_PDF_RE.captures(stripped) {
        ensure_artifacts(job).output_pdf = Some(caps[1].trim().to_string());
    }
    if let Some(caps) = SUMMARY_RE.captures(stripped) {
        ensure_artifacts(job).summary = Some(caps[1].trim().to_string());
    }
    if let Some(caps) = PAGES_PROCESSED_RE.captures(stripped) {
        ensure_artifacts(job).pages_processed = caps[1].parse::<i64>().ok();
    }
    if let Some(caps) = TRANSLATED_ITEMS_RE.captures(stripped) {
        ensure_artifacts(job).translated_items = caps[1].parse::<i64>().ok();
    }
    if let Some(caps) = TRANSLATE_TIME_RE.captures(stripped) {
        ensure_artifacts(job).translate_render_time_seconds = caps[1].parse::<f64>().ok();
    }
    if let Some(caps) = SAVE_TIME_RE.captures(stripped) {
        ensure_artifacts(job).save_time_seconds = caps[1].parse::<f64>().ok();
    }
    if let Some(caps) = TOTAL_TIME_RE.captures(stripped) {
        ensure_artifacts(job).total_time_seconds = caps[1].parse::<f64>().ok();
    }

    if stripped.starts_with("upload done: ") {
        job.stage = Some("mineru_upload".to_string());
        job.stage_detail = Some("文件上传完成，等待 MinerU 处理".to_string());
        return;
    }
    if let Some(caps) = MINERU_BATCH_STATE_RE.captures(stripped) {
        job.stage = Some("mineru_processing".to_string());
        job.stage_detail = Some(format!("MinerU 状态: {}", caps[2].trim()));
        return;
    }
    if stripped.starts_with("layout json: ") {
        job.stage = Some("translation_prepare".to_string());
        job.stage_detail = Some("MinerU 结果已就绪，准备翻译".to_string());
        return;
    }
    if stripped.starts_with("domain-infer: ") {
        job.stage = Some("domain_inference".to_string());
        job.stage_detail = Some("正在识别论文领域".to_string());
        return;
    }
    if stripped.starts_with("continuation-review ") {
        job.stage = Some("continuation_review".to_string());
        job.stage_detail = Some("正在判断跨栏/跨页连续段".to_string());
        return;
    }
    if stripped == "book: page policies start" {
        job.stage = Some("page_policies".to_string());
        job.stage_detail = Some("正在执行块规则、分类和局部拆分".to_string());
        return;
    }
    if let Some(caps) = PAGE_POLICY_MODE_RE.captures(stripped) {
        job.stage = Some("page_policies".to_string());
        job.stage_detail = Some("正在执行块规则、分类和局部拆分".to_string());
        job.progress_current = Some(0);
        job.progress_total = caps[2].parse::<i64>().ok();
        return;
    }
    if let Some(caps) = PAGE_POLICY_PAGE_RE.captures(stripped) {
        let current = caps[1].parse::<i64>().ok();
        let total = caps[2].parse::<i64>().ok();
        let source_page = caps[3].parse::<i64>().unwrap_or(0);
        job.stage = Some("page_policies".to_string());
        job.stage_detail = Some(format!(
            "正在处理第 {}/{} 页策略，对应源文第 {} 页",
            current.unwrap_or(0),
            total.unwrap_or(0),
            source_page
        ));
        job.progress_current = current.map(|v| v.saturating_sub(1));
        job.progress_total = total;
        return;
    }
    if let Some(caps) = TRANSLATE_ATTEMPT_RE.captures(stripped) {
        job.stage = Some("translating".to_string());
        job.stage_detail = Some(format!("正在翻译，第 {}/{} 批", &caps[1], &caps[2]));
        job.progress_current = caps[1].parse::<i64>().ok().map(|v| v.saturating_sub(1));
        job.progress_total = caps[2].parse::<i64>().ok();
        return;
    }
    if let Some(caps) = BATCH_PROGRESS_RE.captures(stripped) {
        job.stage = Some("translating".to_string());
        job.stage_detail = Some(format!("已完成第 {}/{} 批翻译", &caps[1], &caps[2]));
        job.progress_current = caps[1].parse::<i64>().ok();
        job.progress_total = caps[2].parse::<i64>().ok();
        return;
    }
    if stripped.starts_with("render source pdf: ")
        || stripped.starts_with("typst background render selected")
    {
        job.stage = Some("rendering".to_string());
        job.stage_detail = Some("正在准备渲染".to_string());
        return;
    }
    if let Some(caps) = OVERLAY_MERGE_RE.captures(stripped) {
        job.stage = Some("rendering".to_string());
        job.stage_detail = Some(format!("正在渲染第 {}/{} 页", &caps[1], &caps[2]));
        job.progress_current = caps[1].parse::<i64>().ok().map(|v| v.saturating_sub(1));
        job.progress_total = caps[2].parse::<i64>().ok();
        return;
    }
    if stripped.starts_with("save optimized pdf:")
        || stripped.starts_with("image-only compress:")
    {
        job.stage = Some("saving".to_string());
        job.stage_detail = Some("正在保存最终结果".to_string());
    }
}
