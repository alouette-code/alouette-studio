use crate::state::log_to_app_file;
use base64::{engine::general_purpose, Engine as _};
use std::time::Instant;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SqliteColumn {
    pub name: String,
    pub data_type: String,
    pub is_pk: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SqliteTableData {
    pub columns: Vec<SqliteColumn>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub total_rows: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SqliteQueryResult {
    pub success: bool,
    pub columns: Option<Vec<SqliteColumn>>,
    pub rows: Option<Vec<Vec<serde_json::Value>>>,
    pub rows_affected: Option<usize>,
    pub execution_time_ms: Option<u64>,
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn validate_sqlite_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Database path cannot be empty".to_string());
    }
    let path_buf = std::path::Path::new(path);
    let name = path_buf.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.contains("history.sqlite") || name.contains("app.db") || path.contains(".gemini") {
        return Err("Security Block: Access to application database is strictly prohibited.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let path_lower = path.to_lowercase();
        if path_lower.contains("c:\\windows") || path_lower.contains("system32") {
            return Err("Security Block: Access to system folders is strictly prohibited.".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if path.starts_with("/etc")
            || path.starts_with("/var")
            || path.starts_with("/sys")
            || path.starts_with("/proc")
            || path.starts_with("/lib")
        {
            return Err("Security Block: Access to system folders is strictly prohibited.".to_string());
        }
    }

    Ok(())
}

fn validate_identifier_input(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.contains('\0') {
        return Err("Identifier cannot be empty or contain null bytes".to_string());
    }
    Ok(())
}

fn is_valid_table_name(conn: &rusqlite::Connection, table: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type IN ('table', 'view') AND name = ?1;")
        .map_err(|e| e.to_string())?;
    stmt.exists(rusqlite::params![table]).map_err(|e| e.to_string())
}

fn is_valid_column_name(conn: &rusqlite::Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({});", quote_ident(table)))
        .map_err(|e| e.to_string())?;

    let cols_iter = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?;

    for col in cols_iter.flatten() {
        if col == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn value_ref_to_json(val_ref: rusqlite::types::ValueRef<'_>) -> serde_json::Value {
    match val_ref {
        rusqlite::types::ValueRef::Null => serde_json::Value::Null,
        rusqlite::types::ValueRef::Integer(i_val) => serde_json::json!(i_val),
        rusqlite::types::ValueRef::Real(f_val) => serde_json::Number::from_f64(f_val)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(f_val.to_string())),
        rusqlite::types::ValueRef::Text(t_bytes) => {
            serde_json::Value::String(String::from_utf8_lossy(t_bytes).into_owned())
        }
        rusqlite::types::ValueRef::Blob(b_bytes) => {
            let b64 = general_purpose::STANDARD.encode(b_bytes);
            serde_json::Value::String(format!("[Blob: {} bytes] {}", b_bytes.len(), b64))
        }
    }
}

fn json_to_rusqlite(val: serde_json::Value) -> Result<rusqlite::types::Value, String> {
    match val {
        serde_json::Value::Null => Ok(rusqlite::types::Value::Null),
        serde_json::Value::Bool(b) => Ok(rusqlite::types::Value::Integer(if b { 1 } else { 0 })),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(rusqlite::types::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(rusqlite::types::Value::Real(f))
            } else {
                Err("Invalid number type".to_string())
            }
        }
        serde_json::Value::String(s) => Ok(rusqlite::types::Value::Text(s)),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let s = serde_json::to_string(&val).map_err(|e| e.to_string())?;
            Ok(rusqlite::types::Value::Text(s))
        }
    }
}

#[tauri::command]
pub async fn get_sqlite_tables(path: String) -> Result<Vec<String>, String> {
    log_to_app_file(&format!("SQLite: get_sqlite_tables for {}", path));
    validate_sqlite_path(&path)?;

    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| format!("Failed to open database: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;

        let mut tables = Vec::new();
        for r in rows.flatten() {
            tables.push(r);
        }
        Ok(tables)
    })
    .await
    .map_err(|e| format!("SQLite task failed: {}", e))?
}

#[tauri::command]
pub async fn get_sqlite_table_data(
    path: String,
    table: String,
    limit: u32,
    offset: u32,
) -> Result<SqliteTableData, String> {
    log_to_app_file(&format!("SQLite: get_sqlite_table_data for {} -> {}", path, table));
    validate_sqlite_path(&path)?;
    validate_identifier_input(&table)?;

    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| format!("Failed to open database: {}", e))?;

        if !is_valid_table_name(&conn, &table)? {
            return Err("Invalid table name".to_string());
        }

        let safe_table = quote_ident(&table);
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({});", safe_table))
            .map_err(|e| e.to_string())?;

        let cols_iter = stmt
            .query_map([], |row| {
                Ok(SqliteColumn {
                    name: row.get(1)?,
                    data_type: row.get(2)?,
                    is_pk: row.get::<_, i32>(5)? > 0,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut columns = Vec::new();
        for col in cols_iter {
            columns.push(col.map_err(|e| e.to_string())?);
        }

        let total_rows: Option<u64> = conn
            .query_row(&format!("SELECT COUNT(*) FROM {};", safe_table), [], |row| {
                row.get::<_, i64>(0).map(|v| v.max(0) as u64)
            })
            .ok();

        let mut stmt_rows = conn
            .prepare(&format!("SELECT * FROM {} LIMIT ?1 OFFSET ?2;", safe_table))
            .map_err(|e| e.to_string())?;

        let col_count = stmt_rows.column_count();
        let mut rows_iter = stmt_rows
            .query(rusqlite::params![limit as i64, offset as i64])
            .map_err(|e| e.to_string())?;
        let mut rows = Vec::new();

        while let Some(row) = rows_iter.next().map_err(|e| e.to_string())? {
            let mut row_values = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val_ref = row.get_ref(i).map_err(|e| e.to_string())?;
                row_values.push(value_ref_to_json(val_ref));
            }
            rows.push(row_values);
        }

        Ok(SqliteTableData {
            columns,
            rows,
            total_rows,
        })
    })
    .await
    .map_err(|e| format!("SQLite task failed: {}", e))?
}

#[tauri::command]
pub async fn update_sqlite_cell(
    path: String,
    table: String,
    column: String,
    value: serde_json::Value,
    pk_column: String,
    pk_value: serde_json::Value,
) -> Result<(), String> {
    log_to_app_file(&format!(
        "SQLite: update_sqlite_cell for {} -> {}.{} where {} = {:?}",
        path, table, column, pk_column, pk_value
    ));
    validate_sqlite_path(&path)?;
    validate_identifier_input(&table)?;
    validate_identifier_input(&column)?;
    validate_identifier_input(&pk_column)?;

    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        if !is_valid_table_name(&conn, &table)? {
            return Err("Invalid table name".to_string());
        }
        if !is_valid_column_name(&conn, &table, &column)?
            || !is_valid_column_name(&conn, &table, &pk_column)?
        {
            return Err("Invalid column name".to_string());
        }

        let query_str = format!(
            "UPDATE {} SET {} = ?1 WHERE {} = ?2;",
            quote_ident(&table),
            quote_ident(&column),
            quote_ident(&pk_column)
        );

        let mut stmt = conn.prepare(&query_str).map_err(|e| e.to_string())?;
        let db_val = json_to_rusqlite(value)?;
        let db_pk_val = json_to_rusqlite(pk_value)?;

        stmt.execute(rusqlite::params![db_val, db_pk_val])
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("SQLite task failed: {}", e))?
}

#[tauri::command]
pub async fn insert_sqlite_row(path: String, table: String) -> Result<(), String> {
    log_to_app_file(&format!("SQLite: insert_sqlite_row for {} -> {}", path, table));
    validate_sqlite_path(&path)?;
    validate_identifier_input(&table)?;

    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        if !is_valid_table_name(&conn, &table)? {
            return Err("Invalid table name".to_string());
        }

        let safe_table = quote_ident(&table);
        let query_str = format!("INSERT INTO {} DEFAULT VALUES;", safe_table);
        let res = conn.execute(&query_str, []);

        if let Err(e) = res {
            log_to_app_file(&format!(
                "SQLite: DEFAULT VALUES insert failed, trying explicit column null insert. Error: {}",
                e
            ));
            let mut stmt = conn
                .prepare(&format!("PRAGMA table_info({});", safe_table))
                .map_err(|e| e.to_string())?;

            let mut cols = Vec::new();
            let cols_iter = stmt
                .query_map([], |row| {
                    let name: String = row.get(1)?;
                    let pk: i32 = row.get(5)?;
                    Ok((name, pk > 0))
                })
                .map_err(|e| e.to_string())?;

            for c in cols_iter.flatten() {
                if !c.1 {
                    cols.push(c.0);
                }
            }

            if cols.is_empty() {
                return Err("Table has no non-PK columns, could not insert row".to_string());
            }

            let col_names = cols
                .iter()
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = cols.iter().map(|_| "NULL").collect::<Vec<_>>().join(", ");
            let query_str_explicit = format!(
                "INSERT INTO {} ({}) VALUES ({});",
                safe_table, col_names, placeholders
            );
            conn.execute(&query_str_explicit, [])
                .map_err(|err| err.to_string())?;
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("SQLite task failed: {}", e))?
}

#[tauri::command]
pub async fn delete_sqlite_row(
    path: String,
    table: String,
    pk_column: String,
    pk_value: serde_json::Value,
) -> Result<(), String> {
    log_to_app_file(&format!(
        "SQLite: delete_sqlite_row for {} -> {} where {} = {:?}",
        path, table, pk_column, pk_value
    ));
    validate_sqlite_path(&path)?;
    validate_identifier_input(&table)?;
    validate_identifier_input(&pk_column)?;

    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        if !is_valid_table_name(&conn, &table)? {
            return Err("Invalid table name".to_string());
        }
        if !is_valid_column_name(&conn, &table, &pk_column)? {
            return Err("Invalid primary key column".to_string());
        }

        let query_str = format!(
            "DELETE FROM {} WHERE {} = ?1;",
            quote_ident(&table),
            quote_ident(&pk_column)
        );
        let db_pk_val = json_to_rusqlite(pk_value)?;
        conn.execute(&query_str, rusqlite::params![db_pk_val])
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("SQLite task failed: {}", e))?
}

#[tauri::command]
pub async fn add_sqlite_column(
    path: String,
    table: String,
    col_name: String,
    col_type: String,
) -> Result<(), String> {
    log_to_app_file(&format!(
        "SQLite: add_sqlite_column for {} -> {} adding {} {}",
        path, table, col_name, col_type
    ));
    validate_sqlite_path(&path)?;
    validate_identifier_input(&table)?;
    validate_identifier_input(&col_name)?;

    let type_upper = col_type.trim().to_uppercase();
    if !matches!(
        type_upper.as_str(),
        "TEXT" | "INTEGER" | "REAL" | "NUMERIC" | "BLOB" | "BOOLEAN"
    ) {
        return Err(
            "Supported SQLite column types: TEXT, INTEGER, REAL, NUMERIC, BLOB, BOOLEAN"
                .to_string(),
        );
    }

    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        if !is_valid_table_name(&conn, &table)? {
            return Err("Invalid table name".to_string());
        }

        let query_str = format!(
            "ALTER TABLE {} ADD COLUMN {} {};",
            quote_ident(&table),
            quote_ident(&col_name),
            type_upper
        );

        conn.execute(&query_str, []).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("SQLite task failed: {}", e))?
}

#[tauri::command]
pub async fn run_sqlite_query(path: String, query: String) -> Result<SqliteQueryResult, String> {
    log_to_app_file(&format!("SQLite: run_sqlite_query on {}", path));
    validate_sqlite_path(&path)?;

    tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let conn = rusqlite::Connection::open(&path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        let mut stmt = match conn.prepare(&query) {
            Ok(s) => s,
            Err(e) => {
                let err_str = e.to_string();
                let lower = err_str.to_lowercase();
                if lower.contains("multiple statement") || lower.contains("trailing") {
                    conn.execute_batch(&query).map_err(|be| be.to_string())?;
                    return Ok(SqliteQueryResult {
                        success: true,
                        columns: None,
                        rows: None,
                        rows_affected: Some(conn.changes() as usize),
                        execution_time_ms: Some(started.elapsed().as_millis() as u64),
                    });
                }
                return Err(err_str);
            }
        };

        let col_count = stmt.column_count();
        if col_count > 0 {
            let mut columns = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let name = stmt.column_name(i).unwrap_or("").to_string();
                columns.push(SqliteColumn {
                    name,
                    data_type: "ANY".to_string(),
                    is_pk: false,
                });
            }

            let mut rows_iter = stmt.query([]).map_err(|e| e.to_string())?;
            let mut rows = Vec::new();

            while let Some(row) = rows_iter.next().map_err(|e| e.to_string())? {
                let mut row_values = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let val_ref = row.get_ref(i).map_err(|e| e.to_string())?;
                    row_values.push(value_ref_to_json(val_ref));
                }
                rows.push(row_values);
            }

            Ok(SqliteQueryResult {
                success: true,
                columns: Some(columns),
                rows: Some(rows),
                rows_affected: None,
                execution_time_ms: Some(started.elapsed().as_millis() as u64),
            })
        } else {
            let rows_affected = stmt.execute([]).map_err(|e| e.to_string())?;
            Ok(SqliteQueryResult {
                success: true,
                columns: None,
                rows: None,
                rows_affected: Some(rows_affected),
                execution_time_ms: Some(started.elapsed().as_millis() as u64),
            })
        }
    })
    .await
    .map_err(|e| format!("SQLite task failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn sqlite_crud_pagination_and_query_work() {
        let base = std::env::temp_dir().join(format!("alouette_sqlite_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        let db_path = base.join("test_studio.sqlite");
        let path_str = db_path.to_string_lossy().to_string();

        // Create table with hyphenated name and sample records
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE "user-items" (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT,
                    score REAL,
                    raw_blob BLOB
                );
                INSERT INTO "user-items" (title, score, raw_blob) VALUES
                    ('alpha', 1.5, X'CAFE'),
                    ('beta', 2.5, NULL),
                    ('gamma', 3.5, NULL);
                "#,
            )
            .unwrap();
        }

        let tables = get_sqlite_tables(path_str.clone()).await.unwrap();
        assert_eq!(tables, vec!["user-items"]);

        let page1 = get_sqlite_table_data(path_str.clone(), "user-items".into(), 2, 0)
            .await
            .unwrap();
        assert_eq!(page1.total_rows, Some(3));
        assert_eq!(page1.rows.len(), 2);
        assert!(page1.columns[0].is_pk);
        assert_eq!(page1.rows[0][1], json!("alpha"));

        update_sqlite_cell(
            path_str.clone(),
            "user-items".into(),
            "title".into(),
            json!("alpha-updated"),
            "id".into(),
            json!(1),
        )
        .await
        .unwrap();

        insert_sqlite_row(path_str.clone(), "user-items".into())
            .await
            .unwrap();
        let after_insert = get_sqlite_table_data(path_str.clone(), "user-items".into(), 10, 0)
            .await
            .unwrap();
        assert_eq!(after_insert.total_rows, Some(4));
        assert_eq!(after_insert.rows[0][1], json!("alpha-updated"));

        delete_sqlite_row(path_str.clone(), "user-items".into(), "id".into(), json!(4))
            .await
            .unwrap();
        add_sqlite_column(
            path_str.clone(),
            "user-items".into(),
            "active".into(),
            "BOOLEAN".into(),
        )
        .await
        .unwrap();

        let q = run_sqlite_query(
            path_str,
            "SELECT title, score FROM \"user-items\" ORDER BY id ASC".into(),
        )
        .await
        .unwrap();
        assert!(q.success);
        assert_eq!(q.rows.unwrap().len(), 3);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn sqlite_blocks_protected_paths() {
        let res = get_sqlite_tables("/home/user/app.db".into()).await;
        assert!(res.is_err());
    }
}
