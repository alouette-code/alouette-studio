use crate::commands::sqlite::{SqliteQueryResult, SqliteTableData, SqliteColumn};
use duckdb::Connection;

// Define a simple state wrapper if we want to pool duckdb, but since it's embedded, we can just open it.

pub async fn get_duckdb_tables(uri: &str) -> Result<Vec<String>, String> {
    let path = uri.trim_start_matches("duckdb://");
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare("SELECT table_name FROM information_schema.tables WHERE table_schema = 'main'").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| row.get(0)).map_err(|e| e.to_string())?;
    
    let mut tables = Vec::new();
    for name_result in rows {
        if let Ok(name) = name_result {
            tables.push(name);
        }
    }
    
    Ok(tables)
}

pub async fn get_duckdb_table_data(uri: &str, table_name: &str, limit: u32, offset: u32) -> Result<SqliteTableData, String> {
    let path = uri.trim_start_matches("duckdb://");
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    
    // First, get columns
    let query = format!("PRAGMA table_info('{}')", table_name);
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    let mut columns = Vec::new();
    let col_iter = stmt.query_map([], |row| {
        Ok(SqliteColumn {
            name: row.get(1)?,
            data_type: row.get(2)?,
            is_pk: row.get::<_, i32>(5)? > 0,
        })
    }).map_err(|e| e.to_string())?;
    
    for col in col_iter {
        if let Ok(c) = col {
            columns.push(c);
        }
    }
    
    // Now get data
    let data_query = format!("SELECT * FROM \"{}\" LIMIT {} OFFSET {}", table_name, limit, offset);
    let mut data_stmt = conn.prepare(&data_query).map_err(|e| e.to_string())?;
    
    let col_count = data_stmt.column_count();
    let mut rows = Vec::new();
    
    let mut data_rows = data_stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = data_rows.next().map_err(|e| e.to_string())? {
        let mut row_data = Vec::new();
        for i in 0..col_count {
            // Very simplified generic fallback. DuckDB has better ways, but this gets us started.
            let val = match row.get::<_, String>(i) {
                Ok(s) => serde_json::Value::String(s),
                Err(_) => serde_json::Value::Null, // null or unsupported cast
            };
            row_data.push(val);
        }
        rows.push(row_data);
    }
    
    Ok(SqliteTableData {
        columns,
        rows,
    })
}

pub async fn run_duckdb_query(uri: &str, query: &str) -> Result<SqliteQueryResult, String> {
    let path = uri.trim_start_matches("duckdb://");
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    
    // Distinguish between query (SELECT) and execute (INSERT/UPDATE/DELETE)
    let is_select = query.trim_start().to_uppercase().starts_with("SELECT") || 
                    query.trim_start().to_uppercase().starts_with("PRAGMA") || 
                    query.trim_start().to_uppercase().starts_with("SHOW") ||
                    query.trim_start().to_uppercase().starts_with("DESCRIBE");
                    
    if is_select {
        let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
        let col_count = stmt.column_count();
        let column_names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
        
        let mut rows = Vec::new();
        let mut data_rows = stmt.query([]).map_err(|e| e.to_string())?;
        
        while let Some(row) = data_rows.next().map_err(|e| e.to_string())? {
            let mut row_data = Vec::new();
            for i in 0..col_count {
                let val = match row.get::<_, String>(i) {
                    Ok(s) => serde_json::Value::String(s),
                    Err(_) => serde_json::Value::Null,
                };
                row_data.push(val);
            }
            rows.push(row_data);
        }
        
        let columns: Vec<SqliteColumn> = column_names.into_iter().map(|n| SqliteColumn {
            name: n,
            data_type: "UNKNOWN".to_string(),
            is_pk: false,
        }).collect();
        
        Ok(SqliteQueryResult {
            success: true,
            columns: Some(columns),
            rows: Some(rows),
            rows_affected: None,
        })
    } else {
        let affected = conn.execute(query, []).map_err(|e| e.to_string())?;
        Ok(SqliteQueryResult {
            success: true,
            columns: None,
            rows: None,
            rows_affected: Some(affected as usize),
        })
    }
}
