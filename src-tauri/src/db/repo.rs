use crate::db::DbPool;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRecord {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub format: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub added_at: i64,
    pub use_count: i64,
    pub thumbnail_path: Option<String>,
    pub file_hash: Option<String>,
    pub file_size: Option<i64>,
    pub file_modified_time: Option<i64>,
    pub file_status: String,
    pub last_check_time: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TagRecord {
    pub tag_text: String,
    pub is_auto: bool,
}

pub async fn insert_image(pool: &DbPool, rec: &ImageRecord) -> anyhow::Result<()> {
    tracing::debug!("insert_image: id={}", rec.id);
    sqlx::query(
        "INSERT OR IGNORE INTO images(id,file_path,file_name,format,width,height,added_at,use_count,thumbnail_path,
                              file_hash,file_size,file_modified_time,file_status,last_check_time)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)"
    )
    .bind(&rec.id).bind(&rec.file_path).bind(&rec.file_name).bind(&rec.format)
    .bind(rec.width).bind(rec.height).bind(rec.added_at).bind(rec.use_count)
    .bind(&rec.thumbnail_path)
    .bind(&rec.file_hash).bind(rec.file_size).bind(rec.file_modified_time)
    .bind(&rec.file_status).bind(rec.last_check_time)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_image(pool: &DbPool, id: &str) -> anyhow::Result<Option<ImageRecord>> {
    tracing::debug!("get_image: id={id}");
    let row = sqlx::query(
        "SELECT id,file_path,file_name,format,width,height,added_at,use_count,thumbnail_path,
               file_hash,file_size,file_modified_time,file_status,last_check_time
         FROM images WHERE id=?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ImageRecord {
        id: r.get("id"),
        file_path: r.get("file_path"),
        file_name: r.get("file_name"),
        format: r.get("format"),
        width: r.get("width"),
        height: r.get("height"),
        added_at: r.get("added_at"),
        use_count: r.get("use_count"),
        thumbnail_path: r.get("thumbnail_path"),
        file_hash: r.get("file_hash"),
        file_size: r.get("file_size"),
        file_modified_time: r.get("file_modified_time"),
        file_status: r.get("file_status"),
        last_check_time: r.get("last_check_time"),
    }))
}

pub async fn get_image_by_hash(pool: &DbPool, hash: &str) -> anyhow::Result<Option<ImageRecord>> {
    let row = sqlx::query(
        "SELECT id,file_path,file_name,format,width,height,added_at,use_count,thumbnail_path,
               file_hash,file_size,file_modified_time,file_status,last_check_time
         FROM images WHERE file_hash=?1",
    )
    .bind(hash)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| ImageRecord {
        id: r.get("id"),
        file_path: r.get("file_path"),
        file_name: r.get("file_name"),
        format: r.get("format"),
        width: r.get("width"),
        height: r.get("height"),
        added_at: r.get("added_at"),
        use_count: r.get("use_count"),
        thumbnail_path: r.get("thumbnail_path"),
        file_hash: r.get("file_hash"),
        file_size: r.get("file_size"),
        file_modified_time: r.get("file_modified_time"),
        file_status: r.get("file_status"),
        last_check_time: r.get("last_check_time"),
    }))
}

pub async fn delete_image(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    tracing::debug!("delete_image: id={id}");
    let rows = sqlx::query("DELETE FROM images WHERE id=?1")
        .bind(id)
        .execute(pool)
        .await?;
    if rows.rows_affected() == 0 {
        anyhow::bail!("image not found: {id}");
    }
    Ok(())
}

pub async fn clear_all_images(pool: &DbPool) -> anyhow::Result<u64> {
    const BATCH_SIZE: i64 = 1000;
    let mut total_deleted = 0u64;

    loop {
        let rows = sqlx::query("SELECT id FROM images ORDER BY added_at ASC LIMIT ?1")
            .bind(BATCH_SIZE)
            .fetch_all(pool)
            .await?;

        if rows.is_empty() {
            break;
        }

        let ids: Vec<String> = rows.into_iter().map(|row| row.get("id")).collect();
        let mut tx = pool.begin().await?;
        for id in &ids {
            sqlx::query("DELETE FROM ocr_fts WHERE image_id=?1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM images WHERE id=?1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        total_deleted += ids.len() as u64;
    }

    Ok(total_deleted)
}

pub async fn insert_tags(pool: &DbPool, image_id: &str, tags: &[TagRecord]) -> anyhow::Result<()> {
    tracing::info!("[TAGS] Inserting {} tags for {}", tags.len(), image_id);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    for tag in tags {
        let is_auto = tag.is_auto as i64;
        sqlx::query("INSERT INTO tags(image_id,tag_text,is_auto,created_at) VALUES(?1,?2,?3,?4)")
            .bind(image_id)
            .bind(&tag.tag_text)
            .bind(is_auto)
            .bind(now)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_tags(pool: &DbPool, image_id: &str) -> anyhow::Result<()> {
    tracing::info!("[TAGS] Deleting all tags for {}", image_id);
    sqlx::query("DELETE FROM tags WHERE image_id=?1")
        .bind(image_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_embedding(pool: &DbPool, image_id: &str, vector: &[f32]) -> anyhow::Result<()> {
    tracing::debug!(
        "insert_embedding: image_id={image_id}, dims={}",
        vector.len()
    );
    let blob: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
    sqlx::query("INSERT OR REPLACE INTO embeddings(image_id,vector) VALUES(?1,?2)")
        .bind(image_id)
        .bind(&blob)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_all_embeddings(pool: &DbPool) -> anyhow::Result<Vec<(String, Vec<f32>)>> {
    tracing::debug!("get_all_embeddings");
    let rows = sqlx::query("SELECT image_id, vector FROM embeddings")
        .fetch_all(pool)
        .await?;
    let result = rows
        .into_iter()
        .map(|r| {
            let image_id: String = r.get("image_id");
            let blob: Vec<u8> = r.get("vector");
            let vec: Vec<f32> = blob
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            (image_id, vec)
        })
        .collect();
    Ok(result)
}

pub async fn get_embedding(pool: &DbPool, image_id: &str) -> anyhow::Result<Option<Vec<f32>>> {
    let row = sqlx::query("SELECT vector FROM embeddings WHERE image_id = ?1")
        .bind(image_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| {
        let blob: Vec<u8> = r.get("vector");
        blob.chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()
    }))
}

pub async fn insert_ocr(pool: &DbPool, image_id: &str, content: &str) -> anyhow::Result<()> {
    tracing::debug!("insert_ocr: image_id={image_id}, len={}", content.len());
    let mut tx = pool.begin().await?;
    // 先删旧 FTS 条目（避免重复索引），再写普通表和 FTS 虚拟表
    sqlx::query("DELETE FROM ocr_fts WHERE image_id=?1")
        .bind(image_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT OR REPLACE INTO ocr_texts(image_id,content) VALUES(?1,?2)")
        .bind(image_id)
        .bind(content)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO ocr_fts(image_id,content) VALUES(?1,?2)")
        .bind(image_id)
        .bind(content)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete_ocr_for_image(pool: &DbPool, image_id: &str) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM ocr_fts WHERE image_id=?1")
        .bind(image_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM ocr_texts WHERE image_id=?1")
        .bind(image_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get_tag_suggestions(
    pool: &DbPool,
    prefix: &str,
    limit: i64,
) -> anyhow::Result<Vec<String>> {
    tracing::debug!("get_tag_suggestions: prefix={prefix}");
    let pattern = format!("{prefix}%");
    let rows = sqlx::query(
        "SELECT DISTINCT tag_text FROM tags WHERE tag_text LIKE ?1 ORDER BY tag_text LIMIT ?2",
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.get("tag_text")).collect())
}

pub async fn get_images_paged(
    pool: &DbPool,
    page: i64,
    page_size: i64,
) -> anyhow::Result<Vec<ImageRecord>> {
    tracing::debug!("get_images_paged: page={page}");
    let offset = page * page_size;
    let rows = sqlx::query(
        "SELECT id,file_path,file_name,format,width,height,added_at,use_count,thumbnail_path,
               file_hash,file_size,file_modified_time,file_status,last_check_time
         FROM images ORDER BY added_at DESC LIMIT ?1 OFFSET ?2",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ImageRecord {
            id: r.get("id"),
            file_path: r.get("file_path"),
            file_name: r.get("file_name"),
            format: r.get("format"),
            width: r.get("width"),
            height: r.get("height"),
            added_at: r.get("added_at"),
            use_count: r.get("use_count"),
            thumbnail_path: r.get("thumbnail_path"),
            file_hash: r.get("file_hash"),
            file_size: r.get("file_size"),
            file_modified_time: r.get("file_modified_time"),
            file_status: r.get("file_status"),
            last_check_time: r.get("last_check_time"),
        })
        .collect())
}

pub async fn increment_use_count(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    tracing::debug!("increment_use_count: id={id}");
    sqlx::query("UPDATE images SET use_count = use_count + 1 WHERE id=?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_max_use_count(pool: &DbPool) -> anyhow::Result<i64> {
    let row = sqlx::query("SELECT COALESCE(MAX(use_count), 0) as m FROM images")
        .fetch_one(pool)
        .await?;
    Ok(row.get("m"))
}

pub async fn get_use_counts(
    pool: &DbPool,
    ids: &[&str],
) -> anyhow::Result<std::collections::HashMap<String, i64>> {
    // SQLite 不直接支持 IN (?) 参数化多值，逐条查询即可（候选集通常 < 200）
    let mut map = std::collections::HashMap::new();
    for id in ids {
        let row = sqlx::query("SELECT use_count FROM images WHERE id=?1")
            .bind(*id)
            .fetch_optional(pool)
            .await?;
        if let Some(r) = row {
            let uc: i64 = r.get("use_count");
            map.insert(id.to_string(), uc);
        }
    }
    Ok(map)
}

pub async fn has_any_usage(pool: &DbPool) -> anyhow::Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM images WHERE use_count > 0")
        .fetch_one(pool)
        .await?;
    let cnt: i64 = row.get("cnt");
    Ok(cnt > 0)
}

pub async fn get_latest_images(pool: &DbPool, limit: i64) -> anyhow::Result<Vec<ImageRecord>> {
    let rows = sqlx::query(
        "SELECT id,file_path,file_name,format,width,height,added_at,use_count,thumbnail_path,
               file_hash,file_size,file_modified_time,file_status,last_check_time
         FROM images ORDER BY added_at DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ImageRecord {
            id: r.get("id"),
            file_path: r.get("file_path"),
            file_name: r.get("file_name"),
            format: r.get("format"),
            width: r.get("width"),
            height: r.get("height"),
            added_at: r.get("added_at"),
            use_count: r.get("use_count"),
            thumbnail_path: r.get("thumbnail_path"),
            file_hash: r.get("file_hash"),
            file_size: r.get("file_size"),
            file_modified_time: r.get("file_modified_time"),
            file_status: r.get("file_status"),
            last_check_time: r.get("last_check_time"),
        })
        .collect())
}

pub async fn get_all_images(pool: &DbPool) -> anyhow::Result<Vec<ImageRecord>> {
    let rows = sqlx::query(
        "SELECT id,file_path,file_name,format,width,height,added_at,use_count,thumbnail_path,
               file_hash,file_size,file_modified_time,file_status,last_check_time
         FROM images ORDER BY added_at ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ImageRecord {
            id: r.get("id"),
            file_path: r.get("file_path"),
            file_name: r.get("file_name"),
            format: r.get("format"),
            width: r.get("width"),
            height: r.get("height"),
            added_at: r.get("added_at"),
            use_count: r.get("use_count"),
            thumbnail_path: r.get("thumbnail_path"),
            file_hash: r.get("file_hash"),
            file_size: r.get("file_size"),
            file_modified_time: r.get("file_modified_time"),
            file_status: r.get("file_status"),
            last_check_time: r.get("last_check_time"),
        })
        .collect())
}

pub async fn update_file_status(
    pool: &DbPool,
    id: &str,
    status: &str,
    check_time: i64,
) -> anyhow::Result<()> {
    let rows = sqlx::query("UPDATE images SET file_status=?1, last_check_time=?2 WHERE id=?3")
        .bind(status)
        .bind(check_time)
        .bind(id)
        .execute(pool)
        .await?;
    if rows.rows_affected() == 0 {
        anyhow::bail!("image not found: {id}");
    }
    Ok(())
}

pub async fn get_ocr_texts(
    pool: &DbPool,
    ids: &[&str],
) -> anyhow::Result<std::collections::HashMap<String, String>> {
    let mut map = std::collections::HashMap::new();
    for id in ids {
        let row = sqlx::query("SELECT content FROM ocr_texts WHERE image_id=?1")
            .bind(*id)
            .fetch_optional(pool)
            .await?;
        if let Some(r) = row {
            map.insert(id.to_string(), r.get("content"));
        }
    }
    Ok(map)
}

pub async fn get_top_used_images(pool: &DbPool, limit: i64) -> anyhow::Result<Vec<ImageRecord>> {
    let rows = sqlx::query(
        "SELECT id,file_path,file_name,format,width,height,added_at,use_count,thumbnail_path,
               file_hash,file_size,file_modified_time,file_status,last_check_time
         FROM images ORDER BY use_count DESC, added_at DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ImageRecord {
            id: r.get("id"),
            file_path: r.get("file_path"),
            file_name: r.get("file_name"),
            format: r.get("format"),
            width: r.get("width"),
            height: r.get("height"),
            added_at: r.get("added_at"),
            use_count: r.get("use_count"),
            thumbnail_path: r.get("thumbnail_path"),
            file_hash: r.get("file_hash"),
            file_size: r.get("file_size"),
            file_modified_time: r.get("file_modified_time"),
            file_status: r.get("file_status"),
            last_check_time: r.get("last_check_time"),
        })
        .collect())
}

pub async fn get_tags_for_image(pool: &DbPool, image_id: &str) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query("SELECT tag_text FROM tags WHERE image_id=?1 ORDER BY created_at")
        .bind(image_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("tag_text")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    fn make_image(id: &str) -> ImageRecord {
        ImageRecord {
            id: id.to_string(),
            file_path: format!("/tmp/{id}.jpg"),
            file_name: format!("{id}.jpg"),
            format: "jpg".to_string(),
            width: Some(400),
            height: Some(300),
            added_at: 1000,
            use_count: 0,
            thumbnail_path: None,
            file_hash: None,
            file_size: None,
            file_modified_time: None,
            file_status: "normal".to_string(),
            last_check_time: None,
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_insert_and_get_image(pool: SqlitePool) {
        let rec = make_image("img1");
        insert_image(&pool, &rec).await.unwrap();
        let got = get_image(&pool, "img1").await.unwrap().unwrap();
        assert_eq!(got.id, "img1");
        assert_eq!(got.file_path, "/tmp/img1.jpg");
        assert_eq!(got.width, Some(400));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_image_not_found(pool: SqlitePool) {
        let got = get_image(&pool, "nonexistent").await.unwrap();
        assert!(got.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_insert_embedding(pool: SqlitePool) {
        insert_image(&pool, &make_image("img1")).await.unwrap();
        let vec: Vec<f32> = (0..512).map(|i| i as f32 * 0.001).collect();
        insert_embedding(&pool, "img1", &vec).await.unwrap();

        let all = get_all_embeddings(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "img1");
        assert_eq!(all[0].1.len(), 512);
        assert!((all[0].1[0] - 0.0).abs() < 1e-5);
        assert!((all[0].1[1] - 0.001).abs() < 1e-4);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_insert_ocr_and_fts(pool: SqlitePool) {
        insert_image(&pool, &make_image("img1")).await.unwrap();
        insert_ocr(&pool, "img1", "蚌埠住了哈哈哈").await.unwrap();

        let row = sqlx::query("SELECT content FROM ocr_texts WHERE image_id='img1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let content: String = row.get("content");
        assert_eq!(content, "蚌埠住了哈哈哈");

        // trigram tokenizer：子串匹配，无需 * 后缀
        let hits = sqlx::query("SELECT image_id FROM ocr_fts WHERE content MATCH '蚌埠住了'")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(!hits.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_delete_image_cascade(pool: SqlitePool) {
        insert_image(&pool, &make_image("img1")).await.unwrap();
        insert_tags(
            &pool,
            "img1",
            &[TagRecord {
                tag_text: "搞笑".into(),
                is_auto: false,
            }],
        )
        .await
        .unwrap();
        insert_embedding(&pool, "img1", &vec![0.0f32; 512])
            .await
            .unwrap();
        insert_ocr(&pool, "img1", "test").await.unwrap();

        delete_image(&pool, "img1").await.unwrap();

        assert!(get_image(&pool, "img1").await.unwrap().is_none());
        let tags = sqlx::query("SELECT id FROM tags WHERE image_id='img1'")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(tags.is_empty());
        let emb = sqlx::query("SELECT image_id FROM embeddings WHERE image_id='img1'")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(emb.is_empty());
        let ocr = sqlx::query("SELECT image_id FROM ocr_texts WHERE image_id='img1'")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(ocr.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_delete_image_not_found(pool: SqlitePool) {
        let result = delete_image(&pool, "nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_clear_all_images_empty_db(pool: SqlitePool) {
        let deleted = clear_all_images(&pool).await.unwrap();
        assert_eq!(deleted, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_clear_all_images_removes_all(pool: SqlitePool) {
        for i in 0..3 {
            let id = format!("img{i}");
            insert_image(&pool, &make_image(&id)).await.unwrap();
            insert_tags(
                &pool,
                &id,
                &[TagRecord {
                    tag_text: format!("tag{i}"),
                    is_auto: false,
                }],
            )
            .await
            .unwrap();
            insert_embedding(&pool, &id, &vec![i as f32; 512])
                .await
                .unwrap();
            insert_ocr(&pool, &id, &format!("ocr-{i}")).await.unwrap();
        }

        let deleted = clear_all_images(&pool).await.unwrap();
        assert_eq!(deleted, 3);
        assert!(get_all_images(&pool).await.unwrap().is_empty());

        let tags: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM tags")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("cnt");
        let embeddings: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM embeddings")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("cnt");
        let ocr_texts: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM ocr_texts")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("cnt");
        let ocr_fts: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM ocr_fts")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("cnt");

        assert_eq!(tags, 0);
        assert_eq!(embeddings, 0);
        assert_eq!(ocr_texts, 0);
        assert_eq!(ocr_fts, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_clear_all_images_batch_commits(pool: SqlitePool) {
        for i in 0..1500 {
            insert_image(&pool, &make_image(&format!("img{i}")))
                .await
                .unwrap();
        }

        let deleted = clear_all_images(&pool).await.unwrap();
        assert_eq!(deleted, 1500);
        assert!(get_all_images(&pool).await.unwrap().is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_tag_suggestions(pool: SqlitePool) {
        insert_image(&pool, &make_image("img1")).await.unwrap();
        insert_tags(
            &pool,
            "img1",
            &[
                TagRecord {
                    tag_text: "搞笑".into(),
                    is_auto: false,
                },
                TagRecord {
                    tag_text: "搞怪".into(),
                    is_auto: false,
                },
                TagRecord {
                    tag_text: "可爱".into(),
                    is_auto: false,
                },
            ],
        )
        .await
        .unwrap();

        let suggestions = get_tag_suggestions(&pool, "搞", 10).await.unwrap();
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.contains(&"搞笑".to_string()));
        assert!(suggestions.contains(&"搞怪".to_string()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_increment_use_count(pool: SqlitePool) {
        insert_image(&pool, &make_image("img1")).await.unwrap();
        increment_use_count(&pool, "img1").await.unwrap();
        increment_use_count(&pool, "img1").await.unwrap();
        let got = get_image(&pool, "img1").await.unwrap().unwrap();
        assert_eq!(got.use_count, 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_images_paged(pool: SqlitePool) {
        for i in 0..5i64 {
            let mut rec = make_image(&format!("img{i}"));
            rec.added_at = i;
            insert_image(&pool, &rec).await.unwrap();
        }
        let page0 = get_images_paged(&pool, 0, 3).await.unwrap();
        assert_eq!(page0.len(), 3);
        let page1 = get_images_paged(&pool, 1, 3).await.unwrap();
        assert_eq!(page1.len(), 2);
    }

    // ── Phase B：SHA-256 去重 ──────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn test_insert_image_with_hash(pool: SqlitePool) {
        let mut rec = make_image("img1");
        rec.file_hash = Some("abc123".to_string());
        rec.file_size = Some(204800);
        rec.file_modified_time = Some(1700000000);
        insert_image(&pool, &rec).await.unwrap();

        let got = get_image(&pool, "img1").await.unwrap().unwrap();
        assert_eq!(got.file_hash, Some("abc123".to_string()));
        assert_eq!(got.file_size, Some(204800));
        assert_eq!(got.file_modified_time, Some(1700000000));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_image_by_hash_found(pool: SqlitePool) {
        let mut rec = make_image("img1");
        rec.file_hash = Some("deadbeef".to_string());
        insert_image(&pool, &rec).await.unwrap();

        let got = get_image_by_hash(&pool, "deadbeef").await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().id, "img1");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_image_by_hash_not_found(pool: SqlitePool) {
        let got = get_image_by_hash(&pool, "nonexistent_hash").await.unwrap();
        assert!(got.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_null_hash_allows_multiple(pool: SqlitePool) {
        insert_image(&pool, &make_image("img1")).await.unwrap();
        insert_image(&pool, &make_image("img2")).await.unwrap();
        let all = get_images_paged(&pool, 0, 10).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    // ── Phase C：文件状态管理 ──────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn test_default_file_status_is_normal(pool: SqlitePool) {
        insert_image(&pool, &make_image("img1")).await.unwrap();
        let got = get_image(&pool, "img1").await.unwrap().unwrap();
        assert_eq!(got.file_status, "normal");
        assert!(got.last_check_time.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_file_status(pool: SqlitePool) {
        insert_image(&pool, &make_image("img1")).await.unwrap();
        update_file_status(&pool, "img1", "missing", 1700000001)
            .await
            .unwrap();
        let got = get_image(&pool, "img1").await.unwrap().unwrap();
        assert_eq!(got.file_status, "missing");
        assert_eq!(got.last_check_time, Some(1700000001));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_file_status_not_found(pool: SqlitePool) {
        let result = update_file_status(&pool, "nonexistent", "missing", 0).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
