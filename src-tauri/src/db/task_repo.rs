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

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub async fn insert_task(pool: &DbPool, id: &str, file_path: &str) -> anyhow::Result<()> {
    let now = now_secs();
    sqlx::query(
        "INSERT OR IGNORE INTO task_queue(id,file_path,status,created_at,updated_at)
         VALUES(?1,?2,'pending',?3,?4)",
    )
    .bind(id)
    .bind(file_path)
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
    let now = now_secs();
    sqlx::query("UPDATE task_queue SET status=?1, error_message=?2, updated_at=?3 WHERE id=?4")
        .bind(status)
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
         FROM task_queue WHERE status='pending' ORDER BY created_at ASC",
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
    sqlx::query("DELETE FROM task_queue WHERE status IN ('completed','failed')")
        .execute(pool)
        .await?;
    Ok(())
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
        let pending = get_pending_tasks(&pool).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "t3");
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
}
