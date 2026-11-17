use crate::commands::sqlite::{SqliteColumn, SqliteQueryResult, SqliteTableData};
use base64::{engine::general_purpose, Engine as _};
use sqlx::any::AnyPoolOptions;
use sqlx::{AnyPool, Column, Row, TypeInfo, ValueRef};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tauri::State;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

pub struct DbState {
    pub pools: RwLock<HashMap<String, AnyPool>>,
}

impl Default for DbState {
    fn default() -> Self {
        sqlx::any::install_default_drivers();
        Self {
            pools: RwLock::new(HashMap::new()),
        }
    }
}

#[derive(serde::Serialize)]
pub struct DbConnectionResult {
    pub success: bool,
    pub message: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct DbAuthOptions {
    pub uri: String,
    pub auth_type: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub ssl_cert: Option<String>,
    pub ssl_key: Option<String>,
    pub auth_payload: Option<String>,
}

fn is_sqlite_uri(uri: &str) -> bool {
    uri.starts_with("sqlite://") || !uri.contains("://")
}

fn is_mongo_uri(uri: &str) -> bool {
    uri.starts_with("mongodb://") || uri.starts_with("mongodb+srv://")
}

fn is_redis_uri(uri: &str) -> bool {
    uri.starts_with("redis://") || uri.starts_with("rediss://")
}

fn is_kv_uri(uri: &str) -> bool {
    uri.starts_with("sled://") || uri.starts_with("redb://")
}

fn is_mysql_uri(uri: &str) -> bool {
    uri.starts_with("mysql://")
}

fn is_postgres_uri(uri: &str) -> bool {
    uri.starts_with("postgres://") || uri.starts_with("postgresql://")
}

fn quote_sql_ident(uri: &str, name: &str) -> String {
    if is_mysql_uri(uri) {
        format!("`{}`", name.replace('`', "``"))
    } else {
        format!("\"{}\"", name.replace('"', "\"\""))
    }
}

fn escape_sql_string(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn json_to_sql_literal(val: serde_json::Value) -> Result<String, String> {
    match val {
        serde_json::Value::Null => Ok("NULL".to_string()),
        serde_json::Value::Bool(b) => Ok(if b { "TRUE" } else { "FALSE" }.to_string()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => Ok(escape_sql_string(&s)),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let s = serde_json::to_string(&val).map_err(|e| e.to_string())?;
            Ok(escape_sql_string(&s))
        }
    }
}

pub fn build_effective_sql_uri(options: &DbAuthOptions) -> Result<String, String> {
    let raw = options.uri.trim();
    if is_sqlite_uri(raw) {
        return if raw.contains("://") {
            Ok(raw.to_string())
        } else {
            Ok(format!("sqlite://{}", raw))
        };
    }

    let mut parsed =
        url::Url::parse(raw).map_err(|e| format!("Invalid database URI '{}': {}", raw, e))?;

    if let Some(user) = &options.username {
        if !user.trim().is_empty() && parsed.username().is_empty() {
            let _ = parsed.set_username(user.trim());
        }
    }
    if let Some(pass) = &options.password {
        if !pass.is_empty() && parsed.password().is_none() {
            let _ = parsed.set_password(Some(pass));
        }
    }

    if options.auth_type.as_deref() == Some("x509_cert") {
        if let Some(cert_path) = &options.ssl_cert {
            if !cert_path.trim().is_empty() {
                if is_postgres_uri(raw) {
                    parsed
                        .query_pairs_mut()
                        .append_pair("sslmode", "verify-ca")
                        .append_pair("sslrootcert", cert_path.trim());
                } else if is_mysql_uri(raw) {
                    parsed
                        .query_pairs_mut()
                        .append_pair("ssl-mode", "REQUIRED")
                        .append_pair("ssl-ca", cert_path.trim());
                }
            }
        }
    }

    Ok(parsed.to_string())
}

async fn get_or_create_pool(options: &DbAuthOptions, state: &State<'_, DbState>) -> Result<AnyPool, String> {
    let final_uri = build_effective_sql_uri(options)?;

    let pools = state.pools.read().await;
    if let Some(pool) = pools.get(&final_uri) {
        return Ok(pool.clone());
    }
    drop(pools);

    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(&final_uri)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let mut pools = state.pools.write().await;
    pools.insert(final_uri, pool.clone());
    Ok(pool)
}

#[tauri::command]
pub async fn connect_to_db(
    options: DbAuthOptions,
    state: State<'_, DbState>,
) -> Result<DbConnectionResult, String> {
    let uri = options.uri.clone();

    if is_mongo_uri(&uri) {
        return match crate::commands::db_mongo::connect_mongo(options).await {
            Ok(_) => Ok(DbConnectionResult {
                success: true,
                message: "Connected to MongoDB successfully".to_string(),
            }),
            Err(e) => Ok(DbConnectionResult {
                success: false,
                message: e,
            }),
        };
    }

    if is_redis_uri(&uri) {
        return match crate::commands::db_redis::test_redis_connection(&options).await {
            Ok(_) => Ok(DbConnectionResult {
                success: true,
                message: "Connected to Redis successfully".to_string(),
            }),
            Err(e) => Ok(DbConnectionResult {
                success: false,
                message: e,
            }),
        };
    }

    if uri.starts_with("duckdb://") {
        return match crate::commands::db_duckdb::get_duckdb_tables(&uri).await {
            Ok(_) => Ok(DbConnectionResult {
                success: true,
                message: "Connected to DuckDB successfully".to_string(),
            }),
            Err(e) => Ok(DbConnectionResult {
                success: false,
                message: e,
            }),
        };
    }

    if is_kv_uri(&uri) {
        return match crate::commands::db_kv::get_kv_namespaces(&uri).await {
            Ok(_) => Ok(DbConnectionResult {
                success: true,
                message: "Connected to KV store successfully".to_string(),
            }),
            Err(e) => Ok(DbConnectionResult {
                success: false,
                message: e,
            }),
        };
    }

    if is_sqlite_uri(&uri) {
        let path = uri.trim_start_matches("sqlite://").to_string();
        return match crate::commands::sqlite::get_sqlite_tables(path).await {
            Ok(_) => Ok(DbConnectionResult {
                success: true,
                message: "Connected to SQLite successfully".to_string(),
            }),
            Err(e) => Ok(DbConnectionResult {
                success: false,
                message: e,
            }),
        };
    }

    match get_or_create_pool(&options, &state).await {
        Ok(_) => Ok(DbConnectionResult {
            success: true,
            message: "Connected successfully".to_string(),
        }),
        Err(e) => Ok(DbConnectionResult {
            success: false,
            message: e,
        }),
    }
}

fn decode_any_value(row: &sqlx::any::AnyRow, i: usize) -> serde_json::Value {
    if row.try_get_raw(i).map(|r| r.is_null()).unwrap_or(false) {
        return serde_json::Value::Null;
    }

    let col = row.column(i);
    let type_name = col.type_info().name();

    if let Ok(Some(v)) = row.try_get::<Option<bool>, _>(i) {
        serde_json::json!(v)
    } else if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(i) {
        serde_json::json!(v)
    } else if let Ok(Some(v)) = row.try_get::<Option<i32>, _>(i) {
        serde_json::json!(v)
    } else if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(i) {
        serde_json::Number::from_f64(v)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(v.to_string()))
    } else if let Ok(Some(v)) = row.try_get::<Option<String>, _>(i) {
        serde_json::Value::String(v)
    } else if let Ok(Some(v)) = row.try_get::<Option<Vec<u8>>, _>(i) {
        let b64 = general_purpose::STANDARD.encode(&v);
        serde_json::json!(format!("[Blob: {} bytes] {}", v.len(), b64))
    } else {
        serde_json::json!(format!("<Unsupported Type: {}>", type_name))
    }
}

async fn get_sql_schema_columns(
    pool: &AnyPool,
    uri: &str,
    table: &str,
) -> Result<Vec<SqliteColumn>, String> {
    let escaped_table = escape_sql_string(table);

    if is_postgres_uri(uri) {
        let pk_sql = format!(
            "SELECT kcu.column_name FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
             ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema \
             WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_name = {} \
             AND tc.table_schema NOT IN ('pg_catalog', 'information_schema')",
            escaped_table
        );
        let mut pks = HashSet::new();
        if let Ok(pk_rows) = sqlx::query(&pk_sql).fetch_all(pool).await {
            for r in pk_rows {
                if let Ok(name) = r.try_get::<String, _>(0) {
                    pks.insert(name);
                }
            }
        }

        let cols_sql = format!(
            "SELECT column_name, data_type FROM information_schema.columns \
             WHERE table_name = {} AND table_schema NOT IN ('pg_catalog', 'information_schema') \
             ORDER BY ordinal_position",
            escaped_table
        );
        let col_rows = sqlx::query(&cols_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(col_rows
            .into_iter()
            .filter_map(|r| {
                let name = r.try_get::<String, _>(0).ok()?;
                let data_type = r
                    .try_get::<String, _>(1)
                    .unwrap_or_else(|_| "TEXT".to_string());
                Some(SqliteColumn {
                    is_pk: pks.contains(&name),
                    name,
                    data_type,
                })
            })
            .collect());
    }

    if is_mysql_uri(uri) {
        let cols_sql = format!(
            "SELECT COLUMN_NAME, DATA_TYPE, COLUMN_KEY FROM information_schema.columns \
             WHERE table_schema = DATABASE() AND table_name = {} ORDER BY ORDINAL_POSITION",
            escaped_table
        );
        let col_rows = sqlx::query(&cols_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(col_rows
            .into_iter()
            .filter_map(|r| {
                let name = r.try_get::<String, _>(0).ok()?;
                let data_type = r
                    .try_get::<String, _>(1)
                    .unwrap_or_else(|_| "TEXT".to_string());
                let key = r.try_get::<String, _>(2).unwrap_or_default();
                Some(SqliteColumn {
                    name,
                    data_type,
                    is_pk: key.eq_ignore_ascii_case("PRI"),
                })
            })
            .collect());
    }

    Ok(vec![])
}

#[tauri::command]
pub async fn get_db_tables(
    options: DbAuthOptions,
    state: State<'_, DbState>,
) -> Result<Vec<String>, String> {
    let uri = options.uri.clone();
    if is_sqlite_uri(&uri) {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::get_sqlite_tables(path.to_string()).await;
    }
    if is_mongo_uri(&uri) {
        return crate::commands::db_mongo::get_mongo_tables(options).await;
    }
    if is_redis_uri(&uri) {
        return crate::commands::db_redis::get_redis_tables(&options).await;
    }
    if uri.starts_with("duckdb://") {
        return crate::commands::db_duckdb::get_duckdb_tables(&uri).await;
    }
    if is_kv_uri(&uri) {
        return crate::commands::db_kv::get_kv_namespaces(&uri).await;
    }

    let pool = get_or_create_pool(&options, &state).await?;

    let query_str = if is_postgres_uri(&uri) {
        "SELECT tablename AS name FROM pg_catalog.pg_tables WHERE schemaname != 'pg_catalog' AND schemaname != 'information_schema' ORDER BY tablename"
    } else if is_mysql_uri(&uri) {
        "SELECT table_name AS name FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name"
    } else {
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    };

    let rows = match timeout(Duration::from_secs(30), sqlx::query(query_str).fetch_all(&pool)).await
    {
        Ok(res) => res.map_err(|e| e.to_string())?,
        Err(_) => return Err("Query Timeout: Tác vụ vượt quá 30 giây.".to_string()),
    };

    let mut tables = Vec::new();
    for row in rows {
        if let Ok(val) = row.try_get::<String, _>(0) {
            tables.push(val);
        }
    }
    Ok(tables)
}

#[tauri::command]
pub async fn get_db_table_data(
    options: DbAuthOptions,
    table: String,
    limit: u32,
    offset: u32,
    state: State<'_, DbState>,
) -> Result<SqliteTableData, String> {
    let uri = options.uri.clone();
    if is_sqlite_uri(&uri) {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::get_sqlite_table_data(path.to_string(), table, limit, offset)
            .await;
    }
    if is_mongo_uri(&uri) {
        return crate::commands::db_mongo::get_mongo_table_data(options, &table, limit, offset).await;
    }
    if is_redis_uri(&uri) {
        return crate::commands::db_redis::get_redis_table_data(&options, &table, limit, offset)
            .await;
    }
    if uri.starts_with("duckdb://") {
        return crate::commands::db_duckdb::get_duckdb_table_data(&uri, &table, limit, offset)
            .await;
    }
    if is_kv_uri(&uri) {
        return crate::commands::db_kv::get_kv_table_data(&uri, &table, limit, offset).await;
    }

    let pool = get_or_create_pool(&options, &state).await?;
    let safe_table = quote_sql_ident(&uri, &table);

    let mut columns = get_sql_schema_columns(&pool, &uri, &table).await?;

    let count_sql = format!("SELECT COUNT(*) FROM {}", safe_table);
    let total_rows: Option<u64> = sqlx::query(&count_sql)
        .fetch_one(&pool)
        .await
        .ok()
        .and_then(|r| r.try_get::<i64, _>(0).ok())
        .map(|v| v.max(0) as u64);

    let query_str = format!("SELECT * FROM {} LIMIT {} OFFSET {}", safe_table, limit, offset);
    let rows = match timeout(Duration::from_secs(30), sqlx::query(&query_str).fetch_all(&pool)).await
    {
        Ok(res) => res.map_err(|e| e.to_string())?,
        Err(_) => return Err("Query Timeout: Lấy dữ liệu bảng vượt quá 30 giây.".to_string()),
    };

    if columns.is_empty() {
        if let Some(first_row) = rows.first() {
            for col in first_row.columns() {
                columns.push(SqliteColumn {
                    name: col.name().to_string(),
                    data_type: col.type_info().name().to_string(),
                    is_pk: false,
                });
            }
        }
    }

    let mut data = Vec::with_capacity(rows.len());
    for row in rows {
        let mut row_data = Vec::with_capacity(row.columns().len());
        for col in row.columns() {
            row_data.push(decode_any_value(&row, col.ordinal()));
        }
        data.push(row_data);
    }

    Ok(SqliteTableData {
        columns,
        rows: data,
        total_rows,
    })
}

fn is_sql_read_query(query: &str) -> bool {
    let upper = query.trim_start().to_uppercase();
    let first = upper.split_whitespace().next().unwrap_or("");
    matches!(
        first,
        "SELECT" | "WITH" | "SHOW" | "EXPLAIN" | "DESCRIBE" | "DESC" | "PRAGMA" | "VALUES"
    ) || upper.contains(" RETURNING ")
}

#[tauri::command]
pub async fn run_db_query(
    options: DbAuthOptions,
    query: String,
    state: State<'_, DbState>,
) -> Result<SqliteQueryResult, String> {
    let uri = options.uri.clone();
    if is_sqlite_uri(&uri) {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::run_sqlite_query(path.to_string(), query).await;
    }
    if is_mongo_uri(&uri) {
        return crate::commands::db_mongo::run_mongo_query(options, &query).await;
    }
    if is_redis_uri(&uri) {
        return crate::commands::db_redis::run_redis_command(&options, &query).await;
    }
    if uri.starts_with("duckdb://") {
        return crate::commands::db_duckdb::run_duckdb_query(&uri, &query).await;
    }
    if is_kv_uri(&uri) {
        return crate::commands::db_kv::run_kv_query(&uri, &query).await;
    }

    let started = Instant::now();
    let pool = get_or_create_pool(&options, &state).await?;

    if is_sql_read_query(&query) {
        let rows = match timeout(
            Duration::from_secs(45),
            sqlx::query(&query).fetch_all(&pool),
        )
        .await
        {
            Ok(res) => res.map_err(|e| e.to_string())?,
            Err(_) => {
                return Err(
                    "Query Timeout: Truy vấn phức tạp vượt quá 45 giây. Hệ thống tự động hủy để chống treo."
                        .to_string(),
                )
            }
        };

        let mut columns = Vec::new();
        if let Some(first_row) = rows.first() {
            for col in first_row.columns() {
                columns.push(SqliteColumn {
                    name: col.name().to_string(),
                    data_type: col.type_info().name().to_string(),
                    is_pk: false,
                });
            }
        }
        let mut data = Vec::with_capacity(rows.len());
        for row in rows {
            let mut row_data = Vec::with_capacity(row.columns().len());
            for col in row.columns() {
                row_data.push(decode_any_value(&row, col.ordinal()));
            }
            data.push(row_data);
        }

        Ok(SqliteQueryResult {
            success: true,
            columns: Some(columns),
            rows: Some(data),
            rows_affected: None,
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
        })
    } else {
        let res = match timeout(Duration::from_secs(45), sqlx::query(&query).execute(&pool)).await {
            Ok(res) => res.map_err(|e| e.to_string())?,
            Err(_) => return Err("Query Timeout: Lệnh thực thi vượt quá 45 giây.".to_string()),
        };

        Ok(SqliteQueryResult {
            success: true,
            columns: None,
            rows: None,
            rows_affected: Some(res.rows_affected() as usize),
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
        })
    }
}

#[tauri::command]
pub async fn update_db_cell(
    uri: String,
    table: String,
    pk_column: String,
    pk_value: serde_json::Value,
    col_name: String,
    new_value: serde_json::Value,
    state: State<'_, DbState>,
) -> Result<(), String> {
    if is_sqlite_uri(&uri) {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::update_sqlite_cell(
            path.to_string(),
            table,
            col_name,
            new_value,
            pk_column,
            pk_value,
        )
        .await;
    }
    if uri.starts_with("duckdb://") {
        return crate::commands::db_duckdb::update_duckdb_cell(
            &uri, &table, &pk_column, &pk_value, &col_name, &new_value,
        )
        .await;
    }
    if is_redis_uri(&uri) {
        let key = pk_value
            .as_str()
            .ok_or_else(|| "Redis key must be a string".to_string())?;
        let opts = DbAuthOptions {
            uri: uri.clone(),
            auth_type: None,
            username: None,
            password: None,
            ssl_cert: None,
            ssl_key: None,
            auth_payload: None,
        };
        return crate::commands::db_redis::update_redis_cell(&opts, key, &col_name, &new_value)
            .await;
    }
    if is_kv_uri(&uri) {
        let key = pk_value
            .as_str()
            .ok_or_else(|| "KV key must be a string".to_string())?;
        return crate::commands::db_kv::update_kv_value(&uri, &table, key, &new_value).await;
    }
    if is_mongo_uri(&uri) {
        return Err("Inline cell editing on MongoDB documents is not enabled; use Query view to run updates.".to_string());
    }

    let opts = DbAuthOptions {
        uri: uri.clone(),
        auth_type: None,
        username: None,
        password: None,
        ssl_cert: None,
        ssl_key: None,
        auth_payload: None,
    };
    let pool = get_or_create_pool(&opts, &state).await?;
    let val_lit = json_to_sql_literal(new_value)?;
    let pk_lit = json_to_sql_literal(pk_value)?;
    let query = format!(
        "UPDATE {} SET {} = {} WHERE {} = {}",
        quote_sql_ident(&uri, &table),
        quote_sql_ident(&uri, &col_name),
        val_lit,
        quote_sql_ident(&uri, &pk_column),
        pk_lit
    );
    sqlx::query(&query)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_db_row(
    uri: String,
    table: String,
    pk_column: String,
    pk_value: serde_json::Value,
    state: State<'_, DbState>,
) -> Result<(), String> {
    if is_sqlite_uri(&uri) {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::delete_sqlite_row(
            path.to_string(),
            table,
            pk_column,
            pk_value,
        )
        .await;
    }
    if uri.starts_with("duckdb://") {
        return crate::commands::db_duckdb::delete_duckdb_row(&uri, &table, &pk_column, &pk_value)
            .await;
    }
    if is_redis_uri(&uri) {
        let key = pk_value
            .as_str()
            .ok_or_else(|| "Redis key must be a string".to_string())?;
        let opts = DbAuthOptions {
            uri: uri.clone(),
            auth_type: None,
            username: None,
            password: None,
            ssl_cert: None,
            ssl_key: None,
            auth_payload: None,
        };
        return crate::commands::db_redis::delete_redis_key(&opts, key).await;
    }
    if is_kv_uri(&uri) {
        let key = pk_value
            .as_str()
            .ok_or_else(|| "KV key must be a string".to_string())?;
        return crate::commands::db_kv::delete_kv_key(&uri, &table, key).await;
    }
    if is_mongo_uri(&uri) {
        return Err("Row deletion via grid is not enabled for MongoDB; use Query view with {\"collection\":\"...\",\"deleteMany\":{...}}.".to_string());
    }

    let opts = DbAuthOptions {
        uri: uri.clone(),
        auth_type: None,
        username: None,
        password: None,
        ssl_cert: None,
        ssl_key: None,
        auth_payload: None,
    };
    let pool = get_or_create_pool(&opts, &state).await?;
    let pk_lit = json_to_sql_literal(pk_value)?;
    let query = format!(
        "DELETE FROM {} WHERE {} = {}",
        quote_sql_ident(&uri, &table),
        quote_sql_ident(&uri, &pk_column),
        pk_lit
    );
    sqlx::query(&query)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn insert_db_row(
    uri: String,
    table: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    if is_sqlite_uri(&uri) {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::insert_sqlite_row(path.to_string(), table).await;
    }
    if is_mongo_uri(&uri) || is_redis_uri(&uri) || is_kv_uri(&uri) {
        return Err("Empty-row insert does not apply to NoSQL/KV stores; use the Query tab to insert structured records or keys.".to_string());
    }

    let opts = DbAuthOptions {
        uri: uri.clone(),
        auth_type: None,
        username: None,
        password: None,
        ssl_cert: None,
        ssl_key: None,
        auth_payload: None,
    };
    let pool = get_or_create_pool(&opts, &state).await?;
    let safe_table = quote_sql_ident(&uri, &table);
    let query = format!("INSERT INTO {} DEFAULT VALUES", safe_table);
    match sqlx::query(&query).execute(&pool).await {
        Ok(_) => Ok(()),
        Err(_) => {
            let query = format!("INSERT INTO {} () VALUES ()", safe_table);
            sqlx::query(&query)
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn add_db_column(
    uri: String,
    table: String,
    col_name: String,
    col_type: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    if is_sqlite_uri(&uri) {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::add_sqlite_column(
            path.to_string(),
            table,
            col_name,
            col_type,
        )
        .await;
    }
    if uri.starts_with("duckdb://") {
        return crate::commands::db_duckdb::add_duckdb_column(&uri, &table, &col_name, &col_type)
            .await;
    }
    if is_mongo_uri(&uri) || is_redis_uri(&uri) || is_kv_uri(&uri) {
        return Err("Adding fixed schema columns is only applicable to SQL tables.".to_string());
    }

    let type_upper = col_type.trim().to_uppercase();
    if !matches!(
        type_upper.as_str(),
        "TEXT" | "VARCHAR" | "INTEGER" | "BIGINT" | "REAL" | "DOUBLE" | "BOOLEAN" | "JSON" | "BLOB"
    ) {
        return Err(format!("Unsupported column type '{}'", col_type));
    }

    let opts = DbAuthOptions {
        uri: uri.clone(),
        auth_type: None,
        username: None,
        password: None,
        ssl_cert: None,
        ssl_key: None,
        auth_payload: None,
    };
    let pool = get_or_create_pool(&opts, &state).await?;
    let query = format!(
        "ALTER TABLE {} ADD COLUMN {} {}",
        quote_sql_ident(&uri, &table),
        quote_sql_ident(&uri, &col_name),
        type_upper
    );
    sqlx::query(&query)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_effective_sql_uri_injects_auth_and_ssl() {
        let opts = DbAuthOptions {
            uri: "postgres://localhost:5432/mydb".to_string(),
            auth_type: Some("x509_cert".to_string()),
            username: Some("alice".to_string()),
            password: Some("pw".to_string()),
            ssl_cert: Some("/tmp/ca.pem".to_string()),
            ssl_key: None,
            auth_payload: None,
        };
        let uri = build_effective_sql_uri(&opts).unwrap();
        assert!(uri.starts_with("postgres://alice:pw@localhost:5432/mydb?"));
        assert!(uri.contains("sslmode=verify-ca"));
        assert!(uri.contains("sslrootcert=%2Ftmp%2Fca.pem"));
    }

    #[test]
    fn quote_sql_ident_escapes_by_dialect() {
        assert_eq!(quote_sql_ident("mysql://host/db", "my`col"), "`my``col`");
        assert_eq!(quote_sql_ident("postgres://host/db", "my\"col"), "\"my\"\"col\"");
    }

    #[test]
    fn sql_read_query_classifier_handles_cte_and_dml() {
        assert!(is_sql_read_query("  WITH t AS (SELECT 1) SELECT * FROM t"));
        assert!(is_sql_read_query("INSERT INTO users VALUES (1) RETURNING id"));
        assert!(!is_sql_read_query("UPDATE users SET a = 1 WHERE id = 2"));
    }
}
