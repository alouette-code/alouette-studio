use crate::commands::sqlite::{SqliteColumn, SqliteQueryResult, SqliteTableData};
use redb::{ReadableTable, ReadableTableMetadata, TableDefinition, TableHandle};
use std::time::Instant;

fn format_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let preview: String = bytes.iter().take(64).map(|b| format!("{:02x}", b)).collect();
            if bytes.len() > 64 {
                format!("[hex {}B: {}...]", bytes.len(), preview)
            } else {
                format!("[hex {}B: {}]", bytes.len(), preview)
            }
        }
    }
}

pub async fn get_kv_namespaces(uri: &str) -> Result<Vec<String>, String> {
    let uri = uri.to_string();
    tokio::task::spawn_blocking(move || {
        if let Some(path) = uri.strip_prefix("sled://") {
            let db = sled::open(path).map_err(|e| format!("Failed to open sled DB: {}", e))?;
            let mut names: Vec<String> = db
                .tree_names()
                .into_iter()
                .map(|name| String::from_utf8_lossy(&name).into_owned())
                .collect();
            names.sort();
            Ok(names)
        } else if let Some(path) = uri.strip_prefix("redb://") {
            let db = redb::Database::open(path)
                .map_err(|e| format!("Failed to open redb DB: {}", e))?;
            let read_txn = db.begin_read().map_err(|e| e.to_string())?;
            let mut names: Vec<String> = read_txn
                .list_tables()
                .map_err(|e| e.to_string())?
                .map(|h| h.name().to_string())
                .collect();
            names.sort();
            Ok(names)
        } else {
            Err(format!("Unsupported KV URI: {}", uri))
        }
    })
    .await
    .map_err(|e| format!("KV task failed: {}", e))?
}

fn read_redb_typed<K, V, FK, FV>(
    read_txn: &redb::ReadTransaction,
    table_name: &str,
    offset: usize,
    limit: usize,
    fmt_k: FK,
    fmt_v: FV,
) -> Option<(Vec<Vec<serde_json::Value>>, u64)>
where
    K: redb::Key + 'static,
    V: redb::Value + 'static,
    for<'a> FK: Fn(&K::SelfType<'a>) -> String,
    for<'a> FV: Fn(&V::SelfType<'a>) -> serde_json::Value,
{
    let def: TableDefinition<'_, K, V> = TableDefinition::new(table_name);
    let table = read_txn.open_table(def).ok()?;
    let total = table.len().ok()?;
    let iter = table.iter().ok()?;
    let mut rows = Vec::new();
    for entry in iter.skip(offset).take(limit) {
        if let Ok((k, v)) = entry {
            rows.push(vec![
                serde_json::Value::String(fmt_k(&k.value())),
                fmt_v(&v.value()),
            ]);
        }
    }
    Some((rows, total))
}

pub async fn get_kv_table_data(
    uri: &str,
    namespace: &str,
    limit: u32,
    offset: u32,
) -> Result<SqliteTableData, String> {
    let uri = uri.to_string();
    let ns = namespace.to_string();
    let limit_usize = (limit as usize).max(1);
    let offset_usize = offset as usize;

    tokio::task::spawn_blocking(move || {
        let columns = vec![
            SqliteColumn {
                name: "Key".to_string(),
                data_type: "TEXT".to_string(),
                is_pk: true,
            },
            SqliteColumn {
                name: "Value".to_string(),
                data_type: "ANY".to_string(),
                is_pk: false,
            },
        ];

        if let Some(path) = uri.strip_prefix("sled://") {
            let db = sled::open(path).map_err(|e| format!("Failed to open sled DB: {}", e))?;
            let tree = db
                .open_tree(ns.as_bytes())
                .map_err(|e| format!("Failed to open sled tree '{}': {}", ns, e))?;
            let total_rows = Some(tree.len() as u64);
            let mut rows = Vec::new();
            for item in tree.iter().skip(offset_usize).take(limit_usize) {
                let (k, v) = item.map_err(|e| e.to_string())?;
                rows.push(vec![
                    serde_json::Value::String(format_bytes(&k)),
                    serde_json::Value::String(format_bytes(&v)),
                ]);
            }
            return Ok(SqliteTableData {
                columns,
                rows,
                total_rows,
            });
        }

        if let Some(path) = uri.strip_prefix("redb://") {
            let db = redb::Database::open(path)
                .map_err(|e| format!("Failed to open redb DB: {}", e))?;
            let read_txn = db.begin_read().map_err(|e| e.to_string())?;

            // Try common key/value type definitions supported natively by redb
            if let Some((rows, total)) = read_redb_typed::<&str, &str, _, _>(
                &read_txn,
                &ns,
                offset_usize,
                limit_usize,
                |k| k.to_string(),
                |v| serde_json::Value::String(v.to_string()),
            )
            .or_else(|| {
                read_redb_typed::<&str, &[u8], _, _>(
                    &read_txn,
                    &ns,
                    offset_usize,
                    limit_usize,
                    |k| k.to_string(),
                    |v| serde_json::Value::String(format_bytes(v)),
                )
            })
            .or_else(|| {
                read_redb_typed::<&[u8], &[u8], _, _>(
                    &read_txn,
                    &ns,
                    offset_usize,
                    limit_usize,
                    |k| format_bytes(k),
                    |v| serde_json::Value::String(format_bytes(v)),
                )
            })
            .or_else(|| {
                read_redb_typed::<u64, &str, _, _>(
                    &read_txn,
                    &ns,
                    offset_usize,
                    limit_usize,
                    |k| k.to_string(),
                    |v| serde_json::Value::String(v.to_string()),
                )
            })
            .or_else(|| {
                read_redb_typed::<u64, &[u8], _, _>(
                    &read_txn,
                    &ns,
                    offset_usize,
                    limit_usize,
                    |k| k.to_string(),
                    |v| serde_json::Value::String(format_bytes(v)),
                )
            })
            .or_else(|| {
                read_redb_typed::<u64, u64, _, _>(
                    &read_txn,
                    &ns,
                    offset_usize,
                    limit_usize,
                    |k| k.to_string(),
                    |v| serde_json::json!(v),
                )
            }) {
                return Ok(SqliteTableData {
                    columns,
                    rows,
                    total_rows: Some(total),
                });
            }

            // Fallback: untyped metadata inspection when custom Rust Key/Value types were used
            let handles: Vec<_> = read_txn
                .list_tables()
                .map_err(|e| e.to_string())?
                .collect();
            if let Some(handle) = handles.into_iter().find(|h| h.name() == ns) {
                let untyped = read_txn.open_untyped_table(handle).map_err(|e| e.to_string())?;
                let total = untyped.len().unwrap_or(0);
                let stats = untyped.stats().ok();
                let summary = if let Some(st) = stats {
                    format!(
                        "Custom binary Key/Value types (entries: {}, stored_bytes: {}, tree_height: {})",
                        total,
                        st.stored_bytes(),
                        st.tree_height()
                    )
                } else {
                    format!("Custom binary Key/Value types (entries: {})", total)
                };
                return Ok(SqliteTableData {
                    columns,
                    rows: vec![vec![
                        serde_json::Value::String("__redb_table_info__".to_string()),
                        serde_json::Value::String(summary),
                    ]],
                    total_rows: Some(total),
                });
            }

            return Err(format!("redb table '{}' not found", ns));
        }

        Err(format!("Unsupported KV URI: {}", uri))
    })
    .await
    .map_err(|e| format!("KV task failed: {}", e))?
}

pub async fn update_kv_value(
    uri: &str,
    namespace: &str,
    key: &str,
    new_value: &serde_json::Value,
) -> Result<(), String> {
    let uri = uri.to_string();
    let ns = namespace.to_string();
    let key = key.to_string();
    let val_str = match new_value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    };

    tokio::task::spawn_blocking(move || {
        if let Some(path) = uri.strip_prefix("sled://") {
            let db = sled::open(path).map_err(|e| e.to_string())?;
            let tree = db.open_tree(ns.as_bytes()).map_err(|e| e.to_string())?;
            tree.insert(key.as_bytes(), val_str.as_bytes())
                .map_err(|e| e.to_string())?;
            tree.flush().map_err(|e| e.to_string())?;
            return Ok(());
        }
        Err("Inline cell edit for redb requires matching exact compile-time key/value types; use sled or SQL/Redis for direct cell edits.".to_string())
    })
    .await
    .map_err(|e| format!("KV task failed: {}", e))?
}

pub async fn delete_kv_key(uri: &str, namespace: &str, key: &str) -> Result<(), String> {
    let uri = uri.to_string();
    let ns = namespace.to_string();
    let key = key.to_string();

    tokio::task::spawn_blocking(move || {
        if let Some(path) = uri.strip_prefix("sled://") {
            let db = sled::open(path).map_err(|e| e.to_string())?;
            let tree = db.open_tree(ns.as_bytes()).map_err(|e| e.to_string())?;
            tree.remove(key.as_bytes()).map_err(|e| e.to_string())?;
            tree.flush().map_err(|e| e.to_string())?;
            return Ok(());
        }
        Err("Deleting keys is currently supported for sled KV databases.".to_string())
    })
    .await
    .map_err(|e| format!("KV task failed: {}", e))?
}

/// Execute lightweight KV commands on `sled://`:
/// `GET <tree> <key>`, `SET <tree> <key> <value>`, `DEL <tree> <key>`, `PREFIX <tree> <prefix>`
pub async fn run_kv_query(uri: &str, query: &str) -> Result<SqliteQueryResult, String> {
    let uri = uri.to_string();
    let raw = query.trim().to_string();

    tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let Some(path) = uri.strip_prefix("sled://") else {
            return Err("KV query commands are supported on sled:// databases. Use the left table browser for redb://.".to_string());
        };
        let parts: Vec<&str> = raw.splitn(4, char::is_whitespace).collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err("Enter a KV command: GET <tree> <key> | SET <tree> <key> <value> | DEL <tree> <key> | PREFIX <tree> <prefix>".to_string());
        }
        let op = parts[0].to_uppercase();
        let db = sled::open(path).map_err(|e| e.to_string())?;

        match op.as_str() {
            "GET" if parts.len() >= 3 => {
                let tree = db.open_tree(parts[1].as_bytes()).map_err(|e| e.to_string())?;
                let val = tree.get(parts[2].as_bytes()).map_err(|e| e.to_string())?;
                let rows = match val {
                    Some(v) => vec![vec![
                        serde_json::Value::String(parts[2].to_string()),
                        serde_json::Value::String(format_bytes(&v)),
                    ]],
                    None => vec![],
                };
                Ok(SqliteQueryResult {
                    success: true,
                    columns: Some(vec![
                        SqliteColumn { name: "Key".into(), data_type: "TEXT".into(), is_pk: true },
                        SqliteColumn { name: "Value".into(), data_type: "ANY".into(), is_pk: false },
                    ]),
                    rows: Some(rows),
                    rows_affected: None,
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                })
            }
            "PREFIX" if parts.len() >= 2 => {
                let tree = db.open_tree(parts[1].as_bytes()).map_err(|e| e.to_string())?;
                let prefix = parts.get(2).copied().unwrap_or("");
                let mut rows = Vec::new();
                for item in tree.scan_prefix(prefix.as_bytes()).take(500) {
                    let (k, v) = item.map_err(|e| e.to_string())?;
                    rows.push(vec![
                        serde_json::Value::String(format_bytes(&k)),
                        serde_json::Value::String(format_bytes(&v)),
                    ]);
                }
                Ok(SqliteQueryResult {
                    success: true,
                    columns: Some(vec![
                        SqliteColumn { name: "Key".into(), data_type: "TEXT".into(), is_pk: true },
                        SqliteColumn { name: "Value".into(), data_type: "ANY".into(), is_pk: false },
                    ]),
                    rows: Some(rows),
                    rows_affected: None,
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                })
            }
            "SET" if parts.len() >= 4 => {
                let tree = db.open_tree(parts[1].as_bytes()).map_err(|e| e.to_string())?;
                tree.insert(parts[2].as_bytes(), parts[3].as_bytes())
                    .map_err(|e| e.to_string())?;
                tree.flush().map_err(|e| e.to_string())?;
                Ok(SqliteQueryResult {
                    success: true,
                    columns: None,
                    rows: None,
                    rows_affected: Some(1),
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                })
            }
            "DEL" if parts.len() >= 3 => {
                let tree = db.open_tree(parts[1].as_bytes()).map_err(|e| e.to_string())?;
                let existed = tree.remove(parts[2].as_bytes()).map_err(|e| e.to_string())?.is_some();
                tree.flush().map_err(|e| e.to_string())?;
                Ok(SqliteQueryResult {
                    success: true,
                    columns: None,
                    rows: None,
                    rows_affected: Some(if existed { 1 } else { 0 }),
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                })
            }
            _ => Err("Invalid KV command. Examples: GET __sled__default mykey | PREFIX __sled__default user: | SET __sled__default mykey hello | DEL __sled__default mykey".to_string()),
        }
    })
    .await
    .map_err(|e| format!("KV task failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn sled_namespaces_pagination_and_commands_work() {
        let base = std::env::temp_dir().join(format!("alouette_kv_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        let sled_dir = base.join("sled_db");
        let uri = format!("sled://{}", sled_dir.display());

        {
            let db = sled::open(&sled_dir).unwrap();
            let tree = db.open_tree("sessions").unwrap();
            tree.insert(b"k1", b"v1").unwrap();
            tree.insert(b"k2", b"v2").unwrap();
            tree.insert(b"k3", b"v3").unwrap();
            tree.flush().unwrap();
        }

        let ns = get_kv_namespaces(&uri).await.unwrap();
        assert!(ns.contains(&"sessions".to_string()));

        let page = get_kv_table_data(&uri, "sessions", 2, 1).await.unwrap();
        assert_eq!(page.total_rows, Some(3));
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0][0], json!("k2"));

        let q = run_kv_query(&uri, "GET sessions k1").await.unwrap();
        assert_eq!(q.rows.unwrap()[0][1], json!("v1"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn redb_namespaces_and_typed_reading_work() {
        let base = std::env::temp_dir().join(format!("alouette_redb_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        let redb_file = base.join("store.redb");
        let uri = format!("redb://{}", redb_file.display());

        {
            let db = redb::Database::create(&redb_file).unwrap();
            let write_txn = db.begin_write().unwrap();
            {
                let def: TableDefinition<&str, &str> = TableDefinition::new("users");
                let mut t = write_txn.open_table(def).unwrap();
                t.insert("u1", "Alice").unwrap();
                t.insert("u2", "Bob").unwrap();
            }
            write_txn.commit().unwrap();
        }

        let tables = get_kv_namespaces(&uri).await.unwrap();
        assert_eq!(tables, vec!["users"]);

        let data = get_kv_table_data(&uri, "users", 10, 0).await.unwrap();
        assert_eq!(data.total_rows, Some(2));
        assert_eq!(data.rows.len(), 2);
        assert_eq!(data.rows[0], vec![json!("u1"), json!("Alice")]);
        let _ = std::fs::remove_dir_all(&base);
    }
}
