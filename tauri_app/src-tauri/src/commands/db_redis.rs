use crate::commands::database::DbAuthOptions;
use crate::commands::sqlite::{SqliteColumn, SqliteQueryResult, SqliteTableData};
use dashmap::DashMap;
use redis::aio::MultiplexedConnection;
use std::collections::BTreeSet;
use std::sync::OnceLock;
use std::time::Instant;

fn redis_pools() -> &'static DashMap<String, MultiplexedConnection> {
    static REDIS_POOLS: OnceLock<DashMap<String, MultiplexedConnection>> = OnceLock::new();
    REDIS_POOLS.get_or_init(DashMap::new)
}

pub fn build_redis_uri(options: &DbAuthOptions) -> Result<String, String> {
    let mut parsed = url::Url::parse(&options.uri)
        .map_err(|e| format!("Invalid Redis URI '{}': {}", options.uri, e))?;
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
    Ok(parsed.to_string())
}

async fn get_redis_conn(options: &DbAuthOptions) -> Result<MultiplexedConnection, String> {
    let uri = build_redis_uri(options)?;
    if let Some(entry) = redis_pools().get(&uri) {
        return Ok(entry.clone());
    }
    let client =
        redis::Client::open(uri.as_str()).map_err(|e| format!("Redis client error: {}", e))?;
    let manager = client
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| format!("Redis connection failed: {}", e))?;
    redis_pools().insert(uri, manager.clone());
    Ok(manager)
}

pub async fn test_redis_connection(options: &DbAuthOptions) -> Result<(), String> {
    let mut conn = get_redis_conn(options).await?;
    let pong: String = redis::cmd("PING")
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("Redis PING failed: {}", e))?;
    if pong.is_empty() {
        return Err("Empty PING response from Redis".to_string());
    }
    Ok(())
}

async fn scan_keys(
    conn: &mut MultiplexedConnection,
    pattern: &str,
    max_keys: usize,
) -> Result<Vec<String>, String> {
    let mut cursor: u64 = 0;
    let mut keys = Vec::new();
    loop {
        let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(200)
            .query_async(conn)
            .await
            .map_err(|e| format!("Redis SCAN failed: {}", e))?;
        keys.extend(batch);
        if next_cursor == 0 || keys.len() >= max_keys {
            break;
        }
        cursor = next_cursor;
    }
    keys.sort();
    keys.dedup();
    if keys.len() > max_keys {
        keys.truncate(max_keys);
    }
    Ok(keys)
}

pub async fn get_redis_tables(options: &DbAuthOptions) -> Result<Vec<String>, String> {
    let mut conn = get_redis_conn(options).await?;
    let keys = scan_keys(&mut conn, "*", 1000).await?;

    let mut groups = BTreeSet::new();
    for k in &keys {
        if let Some((prefix, _)) = k.split_once(':') {
            if !prefix.is_empty() {
                groups.insert(format!("{}:*", prefix));
            }
        }
    }

    let mut tables = vec!["* (All Keys)".to_string()];
    tables.extend(groups);
    Ok(tables)
}

fn redis_value_to_json(val: redis::Value) -> serde_json::Value {
    match val {
        redis::Value::Nil => serde_json::Value::Null,
        redis::Value::Int(i) => serde_json::json!(i),
        redis::Value::BulkString(bytes) => {
            serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
        }
        redis::Value::SimpleString(s) => serde_json::Value::String(s),
        redis::Value::Okay => serde_json::Value::String("OK".to_string()),
        redis::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(redis_value_to_json).collect())
        }
        redis::Value::Double(d) => serde_json::json!(d),
        redis::Value::Boolean(b) => serde_json::Value::Bool(b),
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}

async fn read_key_preview(conn: &mut MultiplexedConnection, key: &str, key_type: &str) -> String {
    match key_type {
        "string" => redis::cmd("GET")
            .arg(key)
            .query_async::<Option<String>>(conn)
            .await
            .ok()
            .flatten()
            .unwrap_or_default(),
        "hash" => {
            let items: Vec<(String, String)> = redis::cmd("HGETALL")
                .arg(key)
                .query_async(conn)
                .await
                .unwrap_or_default();
            let obj: serde_json::Map<String, serde_json::Value> = items
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            serde_json::Value::Object(obj).to_string()
        }
        "list" => {
            let items: Vec<String> = redis::cmd("LRANGE")
                .arg(key)
                .arg(0)
                .arg(49)
                .query_async(conn)
                .await
                .unwrap_or_default();
            serde_json::json!(items).to_string()
        }
        "set" => {
            let items: Vec<String> = redis::cmd("SMEMBERS")
                .arg(key)
                .query_async(conn)
                .await
                .unwrap_or_default();
            serde_json::json!(items).to_string()
        }
        "zset" => {
            let items: Vec<(String, f64)> = redis::cmd("ZRANGE")
                .arg(key)
                .arg(0)
                .arg(49)
                .arg("WITHSCORES")
                .query_async(conn)
                .await
                .unwrap_or_default();
            serde_json::json!(items).to_string()
        }
        other => format!("<{} value>", other),
    }
}

pub async fn get_redis_table_data(
    options: &DbAuthOptions,
    namespace: &str,
    limit: u32,
    offset: u32,
) -> Result<SqliteTableData, String> {
    let mut conn = get_redis_conn(options).await?;
    let pattern = if namespace == "* (All Keys)" || namespace.trim().is_empty() {
        "*"
    } else {
        namespace
    };

    let all_keys = scan_keys(&mut conn, pattern, 2000).await?;
    let total_rows = Some(all_keys.len() as u64);

    let slice: Vec<String> = all_keys
        .into_iter()
        .skip(offset as usize)
        .take((limit as usize).max(1))
        .collect();

    let mut rows = Vec::with_capacity(slice.len());
    for key in slice {
        let key_type: String = redis::cmd("TYPE")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        let ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .unwrap_or(-1);
        let value_preview = read_key_preview(&mut conn, &key, &key_type).await;

        rows.push(vec![
            serde_json::Value::String(key),
            serde_json::Value::String(key_type),
            serde_json::json!(ttl),
            serde_json::Value::String(value_preview),
        ]);
    }

    Ok(SqliteTableData {
        columns: vec![
            SqliteColumn {
                name: "Key".to_string(),
                data_type: "STRING".to_string(),
                is_pk: true,
            },
            SqliteColumn {
                name: "Type".to_string(),
                data_type: "STRING".to_string(),
                is_pk: false,
            },
            SqliteColumn {
                name: "TTL (s)".to_string(),
                data_type: "INTEGER".to_string(),
                is_pk: false,
            },
            SqliteColumn {
                name: "Value".to_string(),
                data_type: "TEXT".to_string(),
                is_pk: false,
            },
        ],
        rows,
        total_rows,
    })
}

pub async fn update_redis_cell(
    options: &DbAuthOptions,
    key: &str,
    col_name: &str,
    new_value: &serde_json::Value,
) -> Result<(), String> {
    let mut conn = get_redis_conn(options).await?;
    match col_name {
        "Value" => {
            let val_str = match new_value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            };
            let _: String = redis::cmd("SET")
                .arg(key)
                .arg(val_str)
                .query_async(&mut conn)
                .await
                .map_err(|e| format!("Redis SET failed: {}", e))?;
            Ok(())
        }
        "TTL (s)" => {
            let ttl = new_value
                .as_i64()
                .ok_or_else(|| "TTL must be an integer (seconds, or -1 to persist)".to_string())?;
            if ttl < 0 {
                let _: i64 = redis::cmd("PERSIST")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                let _: bool = redis::cmd("EXPIRE")
                    .arg(key)
                    .arg(ttl)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        other => Err(format!("Column '{}' is read-only for Redis keys", other)),
    }
}

pub async fn delete_redis_key(options: &DbAuthOptions, key: &str) -> Result<(), String> {
    let mut conn = get_redis_conn(options).await?;
    let _: i64 = redis::cmd("DEL")
        .arg(key)
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("Redis DEL failed: {}", e))?;
    Ok(())
}

/// Split a Redis CLI command string preserving single/double-quoted tokens.
pub fn tokenize_redis_command(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut in_token = false;

    for ch in input.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => {
                quote = None;
            }
            (Some(_), c) => {
                current.push(c);
            }
            (None, '\'' | '"') => {
                quote = Some(ch);
                in_token = true;
            }
            (None, c) if c.is_whitespace() => {
                if in_token {
                    args.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            (None, c) => {
                current.push(c);
                in_token = true;
            }
        }
    }
    if quote.is_some() {
        return Err("Unterminated quote in Redis command".to_string());
    }
    if in_token {
        args.push(current);
    }
    Ok(args)
}

pub async fn run_redis_command(
    options: &DbAuthOptions,
    query: &str,
) -> Result<SqliteQueryResult, String> {
    let started = Instant::now();
    let tokens = tokenize_redis_command(query.trim())?;
    if tokens.is_empty() {
        return Err("Redis command cannot be empty (e.g. GET key, HGETALL hash, INFO)".to_string());
    }

    let mut conn = get_redis_conn(options).await?;
    let mut cmd = redis::cmd(&tokens[0]);
    for arg in &tokens[1..] {
        cmd.arg(arg);
    }

    let val: redis::Value = cmd
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("Redis error: {}", e))?;

    let json_val = redis_value_to_json(val);
    let rows = match json_val {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .enumerate()
            .map(|(idx, item)| vec![serde_json::json!(idx), item])
            .collect(),
        single => vec![vec![serde_json::Value::String(tokens[0].to_uppercase()), single]],
    };

    Ok(SqliteQueryResult {
        success: true,
        columns: Some(vec![
            SqliteColumn {
                name: "Index / Command".to_string(),
                data_type: "TEXT".to_string(),
                is_pk: false,
            },
            SqliteColumn {
                name: "Result".to_string(),
                data_type: "ANY".to_string(),
                is_pk: false,
            },
        ]),
        rows: Some(rows),
        rows_affected: None,
        execution_time_ms: Some(started.elapsed().as_millis() as u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_handles_quoted_arguments() {
        let tokens = tokenize_redis_command(r#"SET "user:1:name" 'Nguyen Van A'"#).unwrap();
        assert_eq!(tokens, vec!["SET", "user:1:name", "Nguyen Van A"]);
    }

    #[test]
    fn tokenize_rejects_unterminated_quote() {
        assert!(tokenize_redis_command(r#"SET "key val"#).is_err());
    }

    #[test]
    fn build_uri_injects_auth_when_missing() {
        let opts = DbAuthOptions {
            uri: "redis://127.0.0.1:6379/0".to_string(),
            auth_type: None,
            username: Some("admin".to_string()),
            password: Some("s3cr3t".to_string()),
            ssl_cert: None,
            ssl_key: None,
            auth_payload: None,
        };
        let built = build_redis_uri(&opts).unwrap();
        assert_eq!(built, "redis://admin:s3cr3t@127.0.0.1:6379/0");
    }
}
