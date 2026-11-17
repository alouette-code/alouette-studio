use crate::commands::sqlite::{SqliteColumn, SqliteQueryResult, SqliteTableData};
use base64::{engine::general_purpose, Engine as _};
use duckdb::types::ValueRef;
use duckdb::Connection;
use std::collections::HashSet;
use std::time::Instant;

fn duckdb_path(uri: &str) -> &str {
    uri.trim_start_matches("duckdb://")
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn duckdb_val_to_json(val: ValueRef<'_>) -> serde_json::Value {
    match val {
        ValueRef::Null => serde_json::Value::Null,
        ValueRef::Boolean(b) => serde_json::Value::Bool(b),
        ValueRef::TinyInt(n) => serde_json::json!(n),
        ValueRef::SmallInt(n) => serde_json::json!(n),
        ValueRef::Int(n) => serde_json::json!(n),
        ValueRef::BigInt(n) => serde_json::json!(n),
        ValueRef::HugeInt(n) => {
            if let Ok(v) = i64::try_from(n) {
                serde_json::json!(v)
            } else {
                serde_json::Value::String(n.to_string())
            }
        }
        ValueRef::UTinyInt(n) => serde_json::json!(n),
        ValueRef::USmallInt(n) => serde_json::json!(n),
        ValueRef::UInt(n) => serde_json::json!(n),
        ValueRef::UBigInt(n) => serde_json::json!(n),
        ValueRef::Float(f) => serde_json::Number::from_f64(f as f64)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(f.to_string())),
        ValueRef::Double(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(f.to_string())),
        ValueRef::Decimal(d) => serde_json::Value::String(d.to_string()),
        ValueRef::Text(bytes) => {
            serde_json::Value::String(String::from_utf8_lossy(bytes).into_owned())
        }
        ValueRef::Blob(bytes) => {
            let b64 = general_purpose::STANDARD.encode(bytes);
            serde_json::Value::String(format!("[Blob: {} bytes] {}", bytes.len(), b64))
        }
        ValueRef::Date32(days) => {
            if let Some(base) = chrono::NaiveDate::from_ymd_opt(1970, 1, 1) {
                if let Some(d) = base.checked_add_signed(chrono::Duration::days(days as i64)) {
                    return serde_json::Value::String(d.format("%Y-%m-%d").to_string());
                }
            }
            serde_json::Value::String(format!("Date32({})", days))
        }
        ValueRef::Timestamp(unit, raw) => {
            let micros = match unit {
                duckdb::types::TimeUnit::Second => raw.saturating_mul(1_000_000),
                duckdb::types::TimeUnit::Millisecond => raw.saturating_mul(1_000),
                duckdb::types::TimeUnit::Microsecond => raw,
                duckdb::types::TimeUnit::Nanosecond => raw / 1_000,
            };
            if let Some(dt) = chrono::DateTime::from_timestamp_micros(micros) {
                serde_json::Value::String(dt.to_rfc3339())
            } else {
                serde_json::Value::String(raw.to_string())
            }
        }
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}

fn is_valid_duckdb_table(conn: &Connection, table_name: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare("SELECT 1 FROM information_schema.tables WHERE table_schema = 'main' AND table_name = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([table_name]).map_err(|e| e.to_string())?;
    Ok(rows.next().map_err(|e| e.to_string())?.is_some())
}

fn format_duckdb_sql_literal(val: &serde_json::Value) -> Result<String, String> {
    match val {
        serde_json::Value::Null => Ok("NULL".to_string()),
        serde_json::Value::Bool(b) => Ok(if *b { "TRUE" } else { "FALSE" }.to_string()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => Ok(format!("'{}'", s.replace('\'', "''"))),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let s = serde_json::to_string(val).map_err(|e| e.to_string())?;
            Ok(format!("'{}'", s.replace('\'', "''")))
        }
    }
}

pub async fn get_duckdb_tables(uri: &str) -> Result<Vec<String>, String> {
    let path = duckdb_path(uri).to_string();
    tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' ORDER BY table_name")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;

        let mut tables = Vec::new();
        for name in rows.flatten() {
            tables.push(name);
        }
        Ok(tables)
    })
    .await
    .map_err(|e| format!("DuckDB task failed: {}", e))?
}

pub async fn get_duckdb_table_data(
    uri: &str,
    table_name: &str,
    limit: u32,
    offset: u32,
) -> Result<SqliteTableData, String> {
    let path = duckdb_path(uri).to_string();
    let table = table_name.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        if !is_valid_duckdb_table(&conn, &table)? {
            return Err(format!("DuckDB table '{}' not found", table));
        }

        // Gather primary key column names cleanly via duckdb_constraints()
        let mut pk_cols: HashSet<String> = HashSet::new();
        if let Ok(mut pk_stmt) = conn.prepare(
            "SELECT unnest(constraint_column_names) FROM duckdb_constraints() WHERE table_name = ?1 AND constraint_type = 'PRIMARY KEY'",
        ) {
            if let Ok(iter) = pk_stmt.query_map([&table], |row| row.get::<_, String>(0)) {
                for col in iter.flatten() {
                    pk_cols.insert(col);
                }
            }
        }

        let mut col_stmt = conn
            .prepare(
                "SELECT column_name, data_type FROM information_schema.columns WHERE table_schema = 'main' AND table_name = ?1 ORDER BY ordinal_position",
            )
            .map_err(|e| e.to_string())?;
        let col_iter = col_stmt
            .query_map([&table], |row| {
                let name: String = row.get(0)?;
                let data_type: String = row.get(1)?;
                Ok(SqliteColumn {
                    is_pk: pk_cols.contains(&name),
                    name,
                    data_type,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut columns = Vec::new();
        for c in col_iter {
            columns.push(c.map_err(|e| e.to_string())?);
        }

        let safe_table = quote_ident(&table);
        let total_rows: Option<u64> = conn
            .query_row(&format!("SELECT COUNT(*) FROM {}", safe_table), [], |row| {
                row.get::<_, i64>(0).map(|v| v.max(0) as u64)
            })
            .ok();

        let data_query = format!("SELECT * FROM {} LIMIT {} OFFSET {}", safe_table, limit, offset);
        let mut data_stmt = conn.prepare(&data_query).map_err(|e| e.to_string())?;
        let mut data_rows = data_stmt.query([]).map_err(|e| e.to_string())?;
        let col_count = columns.len();
        let mut rows = Vec::new();

        while let Some(row) = data_rows.next().map_err(|e| e.to_string())? {
            let mut row_data = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val_ref = row.get_ref(i).map_err(|e| e.to_string())?;
                row_data.push(duckdb_val_to_json(val_ref));
            }
            rows.push(row_data);
        }

        Ok(SqliteTableData {
            columns,
            rows,
            total_rows,
        })
    })
    .await
    .map_err(|e| format!("DuckDB task failed: {}", e))?
}

fn is_duckdb_read_query(query: &str) -> bool {
    let first_word = query
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_uppercase();
    matches!(
        first_word.as_str(),
        "SELECT" | "WITH" | "PRAGMA" | "SHOW" | "DESCRIBE" | "SUMMARIZE" | "EXPLAIN" | "FROM" | "VALUES" | "TABLE"
    )
}

pub async fn run_duckdb_query(uri: &str, query: &str) -> Result<SqliteQueryResult, String> {
    let path = duckdb_path(uri).to_string();
    let sql = query.to_string();
    tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;

        if is_duckdb_read_query(&sql) {
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut data_rows = stmt.query([]).map_err(|e| e.to_string())?;
            let col_count = data_rows.as_ref().map(|s| s.column_count()).unwrap_or(0);
            let mut column_names = Vec::with_capacity(col_count);
            if let Some(s_ref) = data_rows.as_ref() {
                for i in 0..col_count {
                    let name = s_ref.column_name(i).map(|s| s.to_string()).unwrap_or_else(|_| format!("col_{}", i));
                    column_names.push(name);
                }
            }

            let mut rows = Vec::new();
            while let Some(row) = data_rows.next().map_err(|e| e.to_string())? {
                let mut row_data = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let val_ref = row.get_ref(i).map_err(|e| e.to_string())?;
                    row_data.push(duckdb_val_to_json(val_ref));
                }
                rows.push(row_data);
            }

            let columns: Vec<SqliteColumn> = column_names
                .into_iter()
                .map(|name| SqliteColumn {
                    name,
                    data_type: "ANY".to_string(),
                    is_pk: false,
                })
                .collect();

            Ok(SqliteQueryResult {
                success: true,
                columns: Some(columns),
                rows: Some(rows),
                rows_affected: None,
                execution_time_ms: Some(started.elapsed().as_millis() as u64),
            })
        } else {
            let affected = conn.execute(&sql, []).map_err(|e| e.to_string())?;
            Ok(SqliteQueryResult {
                success: true,
                columns: None,
                rows: None,
                rows_affected: Some(affected),
                execution_time_ms: Some(started.elapsed().as_millis() as u64),
            })
        }
    })
    .await
    .map_err(|e| format!("DuckDB task failed: {}", e))?
}

pub async fn update_duckdb_cell(
    uri: &str,
    table: &str,
    pk_column: &str,
    pk_value: &serde_json::Value,
    col_name: &str,
    new_value: &serde_json::Value,
) -> Result<(), String> {
    let path = duckdb_path(uri).to_string();
    let table = table.to_string();
    let pk_column = pk_column.to_string();
    let col_name = col_name.to_string();
    let val_lit = format_duckdb_sql_literal(new_value)?;
    let pk_lit = format_duckdb_sql_literal(pk_value)?;

    tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        if !is_valid_duckdb_table(&conn, &table)? {
            return Err("Invalid DuckDB table".to_string());
        }
        let sql = format!(
            "UPDATE {} SET {} = {} WHERE {} = {}",
            quote_ident(&table),
            quote_ident(&col_name),
            val_lit,
            quote_ident(&pk_column),
            pk_lit
        );
        conn.execute(&sql, []).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("DuckDB task failed: {}", e))?
}

pub async fn delete_duckdb_row(
    uri: &str,
    table: &str,
    pk_column: &str,
    pk_value: &serde_json::Value,
) -> Result<(), String> {
    let path = duckdb_path(uri).to_string();
    let table = table.to_string();
    let pk_column = pk_column.to_string();
    let pk_lit = format_duckdb_sql_literal(pk_value)?;

    tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        if !is_valid_duckdb_table(&conn, &table)? {
            return Err("Invalid DuckDB table".to_string());
        }
        let sql = format!(
            "DELETE FROM {} WHERE {} = {}",
            quote_ident(&table),
            quote_ident(&pk_column),
            pk_lit
        );
        conn.execute(&sql, []).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("DuckDB task failed: {}", e))?
}

pub async fn add_duckdb_column(
    uri: &str,
    table: &str,
    col_name: &str,
    col_type: &str,
) -> Result<(), String> {
    let path = duckdb_path(uri).to_string();
    let table = table.to_string();
    let col_name = col_name.to_string();
    let col_type = col_type.trim().to_uppercase();
    if !matches!(
        col_type.as_str(),
        "VARCHAR" | "TEXT" | "INTEGER" | "BIGINT" | "DOUBLE" | "BOOLEAN" | "BLOB" | "REAL"
    ) {
        return Err(format!("Unsupported DuckDB column type: {}", col_type));
    }

    tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        if !is_valid_duckdb_table(&conn, &table)? {
            return Err("Invalid DuckDB table".to_string());
        }
        let sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            quote_ident(&table),
            quote_ident(&col_name),
            col_type
        );
        conn.execute(&sql, []).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("DuckDB task failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn duckdb_decodes_all_scalar_types_and_runs_cte() {
        let base = std::env::temp_dir().join(format!("alouette_duckdb_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        let db_path = base.join("analytics.duckdb");
        let uri = format!("duckdb://{}", db_path.display());

        run_duckdb_query(
            &uri,
            "CREATE TABLE events (id INTEGER PRIMARY KEY, score DOUBLE, active BOOLEAN, label VARCHAR, created DATE)",
        )
        .await
        .unwrap();

        run_duckdb_query(
            &uri,
            "INSERT INTO events VALUES (1, 99.5, true, 'click', DATE '2026-01-15'), (2, 42.0, false, 'view', DATE '2026-01-16')",
        )
        .await
        .unwrap();

        let tables = get_duckdb_tables(&uri).await.unwrap();
        assert_eq!(tables, vec!["events"]);

        let data = get_duckdb_table_data(&uri, "events", 10, 0).await.unwrap();
        assert_eq!(data.total_rows, Some(2));
        assert!(data.columns[0].is_pk);
        assert_eq!(data.rows[0][0], json!(1));
        assert_eq!(data.rows[0][1], json!(99.5));
        assert_eq!(data.rows[0][2], json!(true));
        assert_eq!(data.rows[0][3], json!("click"));
        assert_eq!(data.rows[0][4], json!("2026-01-15"));

        // CTE query should return result rows rather than failing execute()
        let cte = run_duckdb_query(
            &uri,
            "WITH high AS (SELECT id, score FROM events WHERE active) SELECT * FROM high",
        )
        .await
        .unwrap();
        assert!(cte.success);
        let cte_rows = cte.rows.unwrap();
        assert_eq!(cte_rows.len(), 1);
        assert_eq!(cte_rows[0][0], json!(1));

        // Update & delete by PK
        update_duckdb_cell(&uri, "events", "id", &json!(1), "label", &json!("tap"))
            .await
            .unwrap();
        delete_duckdb_row(&uri, "events", "id", &json!(2))
            .await
            .unwrap();
        let after = get_duckdb_table_data(&uri, "events", 10, 0).await.unwrap();
        assert_eq!(after.total_rows, Some(1));
        assert_eq!(after.rows[0][3], json!("tap"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
