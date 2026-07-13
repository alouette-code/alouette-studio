use tauri::State;
use sqlx::AnyPool;
use sqlx::any::AnyPoolOptions;
use std::collections::HashMap;
use tokio::sync::RwLock;

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

async fn get_or_create_pool(uri: &str, state: &State<'_, DbState>) -> Result<AnyPool, String> {
    let pools = state.pools.read().await;
    if let Some(pool) = pools.get(uri) {
        return Ok(pool.clone());
    }
    drop(pools);

    let mut final_uri = uri.to_string();
    if !final_uri.contains("://") {
        final_uri = format!("sqlite://{}", final_uri);
    }

    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(&final_uri)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let mut pools = state.pools.write().await;
    pools.insert(uri.to_string(), pool.clone());
    Ok(pool)
}

#[tauri::command]
pub async fn connect_to_db(options: DbAuthOptions, state: State<'_, DbState>) -> Result<DbConnectionResult, String> {
    let uri = options.uri.clone();
    if uri.starts_with("mongodb://") || uri.starts_with("mongodb+srv://") {
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

    match get_or_create_pool(&uri, &state).await {
        Ok(_) => Ok(DbConnectionResult {
            success: true,
            message: "Connected successfully".to_string(),
        }),
        Err(e) => Ok(DbConnectionResult {
            success: false,
            message: e,
        })
    }
}

use sqlx::{Row, Column, TypeInfo};
use crate::commands::sqlite::{SqliteTableData, SqliteColumn, SqliteQueryResult};
use tokio::time::{timeout, Duration};

fn decode_any_value(row: &sqlx::any::AnyRow, i: usize) -> serde_json::Value {
    let col = row.column(i);
    let type_name = col.type_info().name();

    if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(i) {
        serde_json::json!(v)
    } else if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(i) {
        serde_json::json!(v)
    } else if let Ok(Some(v)) = row.try_get::<Option<bool>, _>(i) {
        serde_json::json!(v)
    } else if let Ok(Some(v)) = row.try_get::<Option<String>, _>(i) {
        serde_json::json!(v)
    } else if let Ok(Some(v)) = row.try_get::<Option<Vec<u8>>, _>(i) {
        serde_json::json!(format!("[BLOB {} bytes]", v.len()))
    } else {
        // If it's truly null
        // Let's test if it's null by trying an Option<&str> which returns Ok(None) if null
        // However, in sqlx, if it's null, ANY Option<T> returns Ok(None).
        // Since the first check `try_get::<Option<i64>>` would return Ok(None) if it's NULL,
        // reaching here means it's NOT NULL but an unsupported type that we failed to decode.
        serde_json::json!(format!("<Unsupported Type: {}>", type_name))
    }
}

#[tauri::command]
pub async fn get_db_tables(options: DbAuthOptions, state: State<'_, DbState>) -> Result<Vec<String>, String> {
    let uri = options.uri.clone();
    if uri.starts_with("sqlite://") {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::get_sqlite_tables(path.to_string()).await;
    }
    
    if uri.starts_with("mongodb://") || uri.starts_with("mongodb+srv://") {
        return crate::commands::db_mongo::get_mongo_tables(options).await;
    }

    let pool = get_or_create_pool(&uri, &state).await?;
    
    let query_str = if uri.starts_with("postgres") {
        "SELECT tablename as name FROM pg_catalog.pg_tables WHERE schemaname != 'pg_catalog' AND schemaname != 'information_schema'"
    } else if uri.starts_with("mysql") {
        "SELECT table_name as name FROM information_schema.tables WHERE table_schema = DATABASE()"
    } else {
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    };

    let query_future = sqlx::query(query_str).fetch_all(&pool);
    
    let rows = match timeout(Duration::from_secs(30), query_future).await {
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
pub async fn get_db_table_data(options: DbAuthOptions, table: String, limit: u32, offset: u32, state: State<'_, DbState>) -> Result<SqliteTableData, String> {
    let uri = options.uri.clone();
    if uri.starts_with("sqlite://") {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::get_sqlite_table_data(path.to_string(), table, limit, offset).await;
    }
    
    if uri.starts_with("mongodb://") || uri.starts_with("mongodb+srv://") {
        return crate::commands::db_mongo::get_mongo_table_data(options, &table, limit, offset).await;
    }

    let pool = get_or_create_pool(&uri, &state).await?;

    let query_str = format!("SELECT * FROM \"{}\" LIMIT {} OFFSET {}", table.replace("\"", "\"\""), limit, offset);
    
    let query_future = sqlx::query(&query_str).fetch_all(&pool);
    
    let rows = match timeout(Duration::from_secs(30), query_future).await {
        Ok(res) => res.map_err(|e| e.to_string())?,
        Err(_) => return Err("Query Timeout: Lấy dữ liệu bảng vượt quá 30 giây.".to_string()),
    };
        
    let mut columns = Vec::new();
    let mut data = Vec::new();

    if let Some(first_row) = rows.first() {
        for col in first_row.columns() {
            columns.push(SqliteColumn {
                name: col.name().to_string(),
                data_type: col.type_info().name().to_string(),
                is_pk: false,
            });
        }
    }

    for row in rows {
        let mut row_data = Vec::new();
        for col in row.columns() {
            let i = col.ordinal();
            row_data.push(decode_any_value(&row, i));
        }
        data.push(row_data);
    }
    
    Ok(SqliteTableData {
        columns,
        rows: data,
    })
}

#[tauri::command]
pub async fn run_db_query(options: DbAuthOptions, query: String, state: State<'_, DbState>) -> Result<SqliteQueryResult, String> {
    let uri = options.uri.clone();
    if uri.starts_with("sqlite://") {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::run_sqlite_query(path.to_string(), query).await;
    }

    if uri.starts_with("mongodb://") || uri.starts_with("mongodb+srv://") {
        return crate::commands::db_mongo::run_mongo_query(options, &query).await;
    }

    let pool = get_or_create_pool(&uri, &state).await?;

    let query_future = sqlx::query(&query).fetch_all(&pool);
    let query_result = match timeout(Duration::from_secs(45), query_future).await {
        Ok(res) => res,
        Err(_) => return Err("Query Timeout: Truy vấn phức tạp vượt quá 45 giây. Hệ thống tự động hủy để chống treo.".to_string()),
    };

    match query_result {
        Ok(rows) => {
            let mut columns = Vec::new();
            let mut data = Vec::new();
            if let Some(first_row) = rows.first() {
                for col in first_row.columns() {
                    columns.push(SqliteColumn {
                        name: col.name().to_string(),
                        data_type: col.type_info().name().to_string(),
                        is_pk: false,
                    });
                }
            }
            for row in rows {
                let mut row_data = Vec::new();
                for col in row.columns() {
                    let i = col.ordinal();
                    row_data.push(decode_any_value(&row, i));
                }
                data.push(row_data);
            }
            Ok(SqliteQueryResult {
                success: true,
                columns: Some(columns),
                rows: Some(data),
                rows_affected: None,
            })
        },
        Err(e) => {
            match sqlx::query(&query).execute(&pool).await {
                Ok(result) => {
                    Ok(SqliteQueryResult {
                        success: true,
                        columns: None,
                        rows: None,
                        rows_affected: Some(result.rows_affected() as usize),
                    })
                },
                Err(_) => Err(e.to_string())
            }
        }
    }
}

#[tauri::command]
pub async fn update_db_cell(uri: String, table: String, pk_column: String, pk_value: serde_json::Value, col_name: String, new_value: serde_json::Value, state: State<'_, DbState>) -> Result<(), String> {
    if uri.starts_with("sqlite://") {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::update_sqlite_cell(path.to_string(), table, col_name, new_value, pk_column, pk_value).await;
    }

    let pool = get_or_create_pool(&uri, &state).await?;
    let val_str = match new_value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
        _ => return Err("Unsupported value type".to_string()),
    };
    let pk_str = match pk_value {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
        _ => return Err("Unsupported pk type".to_string()),
    };
    let query = format!("UPDATE \"{}\" SET \"{}\" = {} WHERE \"{}\" = {}", table.replace("\"", "\"\""), col_name.replace("\"", "\"\""), val_str, pk_column.replace("\"", "\"\""), pk_str);
    sqlx::query(&query).execute(&pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_db_row(uri: String, table: String, pk_column: String, pk_value: serde_json::Value, state: State<'_, DbState>) -> Result<(), String> {
    if uri.starts_with("sqlite://") {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::delete_sqlite_row(path.to_string(), table, pk_column, pk_value).await;
    }

    let pool = get_or_create_pool(&uri, &state).await?;
    let pk_str = match pk_value {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
        _ => return Err("Unsupported pk type".to_string()),
    };
    let query = format!("DELETE FROM \"{}\" WHERE \"{}\" = {}", table.replace("\"", "\"\""), pk_column.replace("\"", "\"\""), pk_str);
    sqlx::query(&query).execute(&pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn insert_db_row(uri: String, table: String, state: State<'_, DbState>) -> Result<(), String> {
    if uri.starts_with("sqlite://") {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::insert_sqlite_row(path.to_string(), table).await;
    }

    let pool = get_or_create_pool(&uri, &state).await?;
    let query = format!("INSERT INTO \"{}\" DEFAULT VALUES", table.replace("\"", "\"\""));
    match sqlx::query(&query).execute(&pool).await {
        Ok(_) => Ok(()),
        Err(_) => {
            let query = format!("INSERT INTO \"{}\" () VALUES ()", table.replace("\"", "\"\""));
            sqlx::query(&query).execute(&pool).await.map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn add_db_column(uri: String, table: String, col_name: String, col_type: String, state: State<'_, DbState>) -> Result<(), String> {
    if uri.starts_with("sqlite://") {
        let path = uri.trim_start_matches("sqlite://");
        return crate::commands::sqlite::add_sqlite_column(path.to_string(), table, col_name, col_type).await;
    }

    let pool = get_or_create_pool(&uri, &state).await?;
    let query = format!("ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}", table.replace("\"", "\"\""), col_name.replace("\"", "\"\""), col_type);
    sqlx::query(&query).execute(&pool).await.map_err(|e| e.to_string())?;
    Ok(())
}
