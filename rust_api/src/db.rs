use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::models::{StoredJob, UploadRecord};

#[derive(Clone)]
pub struct Db {
    path: PathBuf,
}

impl Db {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS uploads (
                upload_id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                stored_path TEXT NOT NULL,
                bytes INTEGER NOT NULL,
                page_count INTEGER NOT NULL,
                uploaded_at TEXT NOT NULL,
                developer_mode INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS jobs (
                job_id TEXT PRIMARY KEY,
                workflow TEXT NOT NULL,
                status_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                started_at TEXT,
                finished_at TEXT,
                upload_id TEXT,
                pid INTEGER,
                command_json TEXT NOT NULL,
                request_json TEXT NOT NULL,
                error TEXT,
                stage TEXT,
                stage_detail TEXT,
                progress_current INTEGER,
                progress_total INTEGER,
                log_tail_json TEXT NOT NULL,
                result_json TEXT,
                artifacts_json TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_jobs_updated_at ON jobs(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_jobs_upload_id ON jobs(upload_id);
            "#,
        )?;
        Ok(conn)
    }

    pub fn init(&self) -> Result<()> {
        self.connect().map(|_| ())
    }

    pub fn save_upload(&self, upload: &UploadRecord) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            r#"
            INSERT INTO uploads (
                upload_id, filename, stored_path, bytes, page_count, uploaded_at, developer_mode
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(upload_id) DO UPDATE SET
                filename=excluded.filename,
                stored_path=excluded.stored_path,
                bytes=excluded.bytes,
                page_count=excluded.page_count,
                uploaded_at=excluded.uploaded_at,
                developer_mode=excluded.developer_mode
            "#,
            params![
                upload.upload_id,
                upload.filename,
                upload.stored_path,
                upload.bytes as i64,
                upload.page_count as i64,
                upload.uploaded_at,
                if upload.developer_mode { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn get_upload(&self, upload_id: &str) -> Result<UploadRecord> {
        let conn = self.connect()?;
        let upload = conn.query_row(
            "SELECT upload_id, filename, stored_path, bytes, page_count, uploaded_at, developer_mode FROM uploads WHERE upload_id = ?1",
            params![upload_id],
            |row| {
                Ok(UploadRecord {
                    upload_id: row.get(0)?,
                    filename: row.get(1)?,
                    stored_path: row.get(2)?,
                    bytes: row.get::<_, i64>(3)? as u64,
                    page_count: row.get::<_, i64>(4)? as u32,
                    uploaded_at: row.get(5)?,
                    developer_mode: row.get::<_, i64>(6)? != 0,
                })
            },
        ).with_context(|| format!("upload not found: {upload_id}"))?;
        Ok(upload)
    }

    pub fn save_job(&self, job: &StoredJob) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            r#"
            INSERT INTO jobs (
                job_id, workflow, status_json, created_at, updated_at, started_at, finished_at,
                upload_id, pid, command_json, request_json, error, stage, stage_detail,
                progress_current, progress_total, log_tail_json, result_json, artifacts_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(job_id) DO UPDATE SET
                workflow=excluded.workflow,
                status_json=excluded.status_json,
                created_at=excluded.created_at,
                updated_at=excluded.updated_at,
                started_at=excluded.started_at,
                finished_at=excluded.finished_at,
                upload_id=excluded.upload_id,
                pid=excluded.pid,
                command_json=excluded.command_json,
                request_json=excluded.request_json,
                error=excluded.error,
                stage=excluded.stage,
                stage_detail=excluded.stage_detail,
                progress_current=excluded.progress_current,
                progress_total=excluded.progress_total,
                log_tail_json=excluded.log_tail_json,
                result_json=excluded.result_json,
                artifacts_json=excluded.artifacts_json
            "#,
            params![
                job.job_id,
                serde_json::to_string(&job.workflow)?,
                serde_json::to_string(&job.status)?,
                job.created_at,
                job.updated_at,
                job.started_at,
                job.finished_at,
                job.upload_id,
                job.pid.map(|v| v as i64),
                serde_json::to_string(&job.command)?,
                serde_json::to_string(&job.request_payload)?,
                job.error,
                job.stage,
                job.stage_detail,
                job.progress_current,
                job.progress_total,
                serde_json::to_string(&job.log_tail)?,
                serde_json::to_string(&job.result)?,
                serde_json::to_string(&job.artifacts)?,
            ],
        )?;
        Ok(())
    }

    pub fn get_job(&self, job_id: &str) -> Result<StoredJob> {
        let conn = self.connect()?;
        let job = conn
            .query_row(
                r#"
                SELECT
                    job_id, workflow, status_json, created_at, updated_at, started_at, finished_at,
                    upload_id, pid, command_json, request_json, error, stage, stage_detail,
                    progress_current, progress_total, log_tail_json, result_json, artifacts_json
                FROM jobs
                WHERE job_id = ?1
                "#,
                params![job_id],
                |row| {
                    let result_json: Option<String> = row.get(17)?;
                    let artifacts_json: Option<String> = row.get(18)?;
                    Ok(StoredJob {
                        job_id: row.get(0)?,
                        workflow: serde_json::from_str::<_>(&row.get::<_, String>(1)?).unwrap(),
                        status: serde_json::from_str::<_>(&row.get::<_, String>(2)?).unwrap(),
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                        started_at: row.get(5)?,
                        finished_at: row.get(6)?,
                        upload_id: row.get(7)?,
                        pid: row.get::<_, Option<i64>>(8)?.map(|v| v as u32),
                        command: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                        request_payload: serde_json::from_str(&row.get::<_, String>(10)?).unwrap(),
                        error: row.get(11)?,
                        stage: row.get(12)?,
                        stage_detail: row.get(13)?,
                        progress_current: row.get(14)?,
                        progress_total: row.get(15)?,
                        log_tail: serde_json::from_str(&row.get::<_, String>(16)?).unwrap_or_default(),
                        result: result_json
                            .and_then(|text| serde_json::from_str(&text).ok())
                            .unwrap_or(None),
                        artifacts: artifacts_json
                            .and_then(|text| serde_json::from_str(&text).ok())
                            .unwrap_or(None),
                    })
                },
            )
            .with_context(|| format!("job not found: {job_id}"))?;
        Ok(job)
    }

    pub fn list_jobs(&self, limit: u32) -> Result<Vec<StoredJob>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                job_id, workflow, status_json, created_at, updated_at, started_at, finished_at,
                upload_id, pid, command_json, request_json, error, stage, stage_detail,
                progress_current, progress_total, log_tail_json, result_json, artifacts_json
            FROM jobs
            ORDER BY updated_at DESC
            LIMIT ?1
            "#,
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let result_json: Option<String> = row.get(17)?;
            let artifacts_json: Option<String> = row.get(18)?;
            Ok(StoredJob {
                job_id: row.get(0)?,
                workflow: serde_json::from_str::<_>(&row.get::<_, String>(1)?).unwrap(),
                status: serde_json::from_str::<_>(&row.get::<_, String>(2)?).unwrap(),
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                started_at: row.get(5)?,
                finished_at: row.get(6)?,
                upload_id: row.get(7)?,
                pid: row.get::<_, Option<i64>>(8)?.map(|v| v as u32),
                command: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                request_payload: serde_json::from_str(&row.get::<_, String>(10)?).unwrap(),
                error: row.get(11)?,
                stage: row.get(12)?,
                stage_detail: row.get(13)?,
                progress_current: row.get(14)?,
                progress_total: row.get(15)?,
                log_tail: serde_json::from_str(&row.get::<_, String>(16)?).unwrap_or_default(),
                result: result_json
                    .and_then(|text| serde_json::from_str(&text).ok())
                    .unwrap_or(None),
                artifacts: artifacts_json
                    .and_then(|text| serde_json::from_str(&text).ok())
                    .unwrap_or(None),
            })
        })?;
        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }
        Ok(jobs)
    }
}
