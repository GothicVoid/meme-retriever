use crate::db::DbPool;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub file_path: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportBatchSummary {
    pub batch_id: String,
    pub total_count: i64,
    pub imported_count: i64,
    pub duplicated_count: i64,
    pub failed_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportBatchFailure {
    pub task_id: String,
    pub file_path: String,
    pub error_message: Option<String>,
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub async fn insert_task(pool: &DbPool, id: &str, file_path: &str) -> anyhow::Result<()> {
    insert_task_with_batch(pool, id, file_path, "").await
}

pub async fn insert_task_with_batch(
    pool: &DbPool,
    id: &str,
    file_path: &str,
    batch_id: &str,
) -> anyhow::Result<()> {
    let now = now_secs();
    sqlx::query(
        "INSERT OR IGNORE INTO task_queue(id,file_path,status,batch_id,created_at,updated_at)
         VALUES(?1,?2,'pending',NULLIF(?3, ''),?4,?5)",
    )
    .bind(id)
    .bind(file_path)
    .bind(batch_id)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_task_status(
    pool: &DbPool,
    id: &str,
    status: &str,
    error: Option<&str>,
) -> anyhow::Result<()> {
    update_task_status_with_result(pool, id, status, None, error).await
}

pub async fn update_task_status_with_result(
    pool: &DbPool,
    id: &str,
    status: &str,
    result_kind: Option<&str>,
    error: Option<&str>,
) -> anyhow::Result<()> {
    let now = now_secs();
    sqlx::query(
        "UPDATE task_queue
         SET status=?1, result_kind=COALESCE(?2, result_kind), error_message=?3, updated_at=?4
         WHERE id=?5",
    )
    .bind(status)
    .bind(result_kind)
    .bind(error)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_pending_tasks(pool: &DbPool) -> anyhow::Result<Vec<TaskRecord>> {
    let rows = sqlx::query(
        "SELECT id,file_path,status,error_message,created_at,updated_at
         FROM task_queue
         WHERE status IN ('pending', 'processing')
         ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| TaskRecord {
            id: r.get("id"),
            file_path: r.get("file_path"),
            status: r.get("status"),
            error_message: r.get("error_message"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect())
}

pub async fn reset_stale_tasks(pool: &DbPool) -> anyhow::Result<()> {
    let now = now_secs();
    sqlx::query("UPDATE task_queue SET status='pending', updated_at=?1 WHERE status='processing'")
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear_task_queue(pool: &DbPool) -> anyhow::Result<()> {
    sqlx::query(
        "DELETE FROM task_queue WHERE status IN ('pending','processing','completed','failed')",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_pending_task_count(pool: &DbPool) -> anyhow::Result<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM task_queue WHERE status IN ('pending', 'processing')",
    )
    .fetch_one(pool)
    .await?;
    Ok(row.get("cnt"))
}

pub async fn get_latest_import_batch_summary(
    pool: &DbPool,
) -> anyhow::Result<Option<ImportBatchSummary>> {
    let row = sqlx::query(
        "SELECT
            batch_id,
            COUNT(*) AS total_count,
            SUM(CASE WHEN result_kind='imported' THEN 1 ELSE 0 END) AS imported_count,
            SUM(CASE WHEN result_kind='duplicated' THEN 1 ELSE 0 END) AS duplicated_count,
            SUM(CASE WHEN result_kind='failed' THEN 1 ELSE 0 END) AS failed_count
         FROM task_queue
         WHERE batch_id = (
            SELECT batch_id
            FROM task_queue
            WHERE batch_id IS NOT NULL
            ORDER BY created_at DESC, updated_at DESC, batch_id DESC
            LIMIT 1
         )
         GROUP BY batch_id",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ImportBatchSummary {
        batch_id: r.get("batch_id"),
        total_count: r.get("total_count"),
        imported_count: r.get("imported_count"),
        duplicated_count: r.get("duplicated_count"),
        failed_count: r.get("failed_count"),
    }))
}

pub async fn get_import_batch_failures(
    pool: &DbPool,
    batch_id: &str,
) -> anyhow::Result<Vec<ImportBatchFailure>> {
    let rows = sqlx::query(
        "SELECT id, file_path, error_message
         FROM task_queue
         WHERE batch_id=?1 AND status='failed'
         ORDER BY created_at ASC, id ASC",
    )
    .bind(batch_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ImportBatchFailure {
            task_id: r.get("id"),
            file_path: r.get("file_path"),
            error_message: r.get("error_message"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "./migrations")]
    async fn test_insert_and_get_pending(pool: SqlitePool) {
        insert_task(&pool, "task-1", "/tmp/a.jpg").await.unwrap();
        insert_task(&pool, "task-2", "/tmp/b.jpg").await.unwrap();
        let pending = get_pending_tasks(&pool).await.unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|t| t.status == "pending"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_task_completed(pool: SqlitePool) {
        insert_task(&pool, "task-1", "/tmp/a.jpg").await.unwrap();
        update_task_status(&pool, "task-1", "completed", None)
            .await
            .unwrap();
        let pending = get_pending_tasks(&pool).await.unwrap();
        assert!(pending.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_task_failed_with_message(pool: SqlitePool) {
        insert_task(&pool, "task-1", "/tmp/a.jpg").await.unwrap();
        update_task_status(&pool, "task-1", "failed", Some("OCR timeout"))
            .await
            .unwrap();
        let row = sqlx::query("SELECT status, error_message FROM task_queue WHERE id='task-1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let status: String = row.get("status");
        let msg: Option<String> = row.get("error_message");
        assert_eq!(status, "failed");
        assert_eq!(msg.as_deref(), Some("OCR timeout"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_clear_task_queue(pool: SqlitePool) {
        insert_task(&pool, "t1", "/tmp/a.jpg").await.unwrap();
        insert_task(&pool, "t2", "/tmp/b.jpg").await.unwrap();
        insert_task(&pool, "t3", "/tmp/c.jpg").await.unwrap();
        update_task_status(&pool, "t1", "completed", None)
            .await
            .unwrap();
        update_task_status(&pool, "t2", "failed", Some("err"))
            .await
            .unwrap();
        clear_task_queue(&pool).await.unwrap();
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM task_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        let count: i64 = row.get("cnt");
        assert_eq!(count, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_clear_task_queue_also_removes_pending_tasks(pool: SqlitePool) {
        insert_task(&pool, "t1", "/tmp/a.jpg").await.unwrap();
        insert_task(&pool, "t2", "/tmp/b.jpg").await.unwrap();
        update_task_status(&pool, "t2", "processing", None)
            .await
            .unwrap();

        clear_task_queue(&pool).await.unwrap();

        let row = sqlx::query("SELECT COUNT(*) as cnt FROM task_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        let count: i64 = row.get("cnt");
        assert_eq!(count, 0, "放弃任务后不应保留 pending / processing 任务");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_reset_stale_tasks(pool: SqlitePool) {
        insert_task(&pool, "t1", "/tmp/a.jpg").await.unwrap();
        update_task_status(&pool, "t1", "processing", None)
            .await
            .unwrap();
        reset_stale_tasks(&pool).await.unwrap();
        let pending = get_pending_tasks(&pool).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, "pending");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_pending_tasks_includes_processing(pool: SqlitePool) {
        insert_task(&pool, "t1", "/tmp/a.jpg").await.unwrap();
        insert_task(&pool, "t2", "/tmp/b.jpg").await.unwrap();
        update_task_status(&pool, "t2", "processing", None)
            .await
            .unwrap();
        update_task_status(&pool, "t1", "completed", None)
            .await
            .unwrap();
        insert_task(&pool, "t3", "/tmp/c.jpg").await.unwrap();

        let pending = get_pending_tasks(&pool).await.unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().any(|task| task.id == "t2" && task.status == "processing"));
        assert!(pending.iter().any(|task| task.id == "t3" && task.status == "pending"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_latest_import_batch_summary_returns_latest_batch(pool: SqlitePool) {
        insert_task_with_batch(&pool, "a1", "/tmp/a1.jpg", "batch-a")
            .await
            .unwrap();
        insert_task_with_batch(&pool, "a2", "/tmp/a2.jpg", "batch-a")
            .await
            .unwrap();
        update_task_status_with_result(&pool, "a1", "completed", Some("imported"), None)
            .await
            .unwrap();
        update_task_status_with_result(&pool, "a2", "failed", Some("failed"), Some("损坏"))
            .await
            .unwrap();
        sqlx::query(
            "UPDATE task_queue SET created_at=100, updated_at=100 WHERE batch_id='batch-a'",
        )
        .execute(&pool)
        .await
        .unwrap();

        insert_task_with_batch(&pool, "b1", "/tmp/b1.jpg", "batch-b")
            .await
            .unwrap();
        insert_task_with_batch(&pool, "b2", "/tmp/b2.jpg", "batch-b")
            .await
            .unwrap();
        insert_task_with_batch(&pool, "b3", "/tmp/b3.jpg", "batch-b")
            .await
            .unwrap();
        update_task_status_with_result(&pool, "b1", "completed", Some("imported"), None)
            .await
            .unwrap();
        update_task_status_with_result(&pool, "b2", "completed", Some("duplicated"), None)
            .await
            .unwrap();
        update_task_status_with_result(&pool, "b3", "failed", Some("failed"), Some("找不到文件"))
            .await
            .unwrap();
        sqlx::query(
            "UPDATE task_queue SET created_at=200, updated_at=200 WHERE batch_id='batch-b'",
        )
        .execute(&pool)
        .await
        .unwrap();

        let summary = get_latest_import_batch_summary(&pool)
            .await
            .unwrap()
            .expect("should have latest import batch");

        assert_eq!(summary.batch_id, "batch-b");
        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.imported_count, 1);
        assert_eq!(summary.duplicated_count, 1);
        assert_eq!(summary.failed_count, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_import_batch_failures_returns_only_failed_items(pool: SqlitePool) {
        insert_task_with_batch(&pool, "a1", "/tmp/a1.jpg", "batch-a")
            .await
            .unwrap();
        insert_task_with_batch(&pool, "a2", "/tmp/a2.jpg", "batch-a")
            .await
            .unwrap();
        update_task_status_with_result(&pool, "a1", "failed", Some("failed"), Some("损坏"))
            .await
            .unwrap();
        update_task_status_with_result(&pool, "a2", "completed", Some("imported"), None)
            .await
            .unwrap();

        let failures = get_import_batch_failures(&pool, "batch-a").await.unwrap();

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].task_id, "a1");
        assert_eq!(failures[0].file_path, "/tmp/a1.jpg");
        assert_eq!(failures[0].error_message.as_deref(), Some("损坏"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_pending_task_count_counts_pending_and_processing(pool: SqlitePool) {
        insert_task(&pool, "t1", "/tmp/a.jpg").await.unwrap();
        insert_task(&pool, "t2", "/tmp/b.jpg").await.unwrap();
        update_task_status(&pool, "t2", "processing", None)
            .await
            .unwrap();
        update_task_status(&pool, "t1", "completed", None)
            .await
            .unwrap();
        insert_task(&pool, "t3", "/tmp/c.jpg").await.unwrap();

        let count = get_pending_task_count(&pool).await.unwrap();
        assert_eq!(count, 2);
    }
}
