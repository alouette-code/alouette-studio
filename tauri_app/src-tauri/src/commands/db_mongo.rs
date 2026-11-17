use crate::commands::database::DbAuthOptions;
use crate::commands::sqlite::{SqliteColumn, SqliteQueryResult, SqliteTableData};
use dashmap::DashMap;
use futures_util::stream::StreamExt;
use mongodb::bson::{doc, Bson, Document};
use mongodb::{options::ClientOptions, Client};
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Instant;

fn mongo_clients() -> &'static DashMap<String, Client> {
    static MONGO_CLIENTS: OnceLock<DashMap<String, Client>> = OnceLock::new();
    MONGO_CLIENTS.get_or_init(DashMap::new)
}

pub fn build_mongo_uri(options: &DbAuthOptions) -> Result<String, String> {
    let mut parsed = url::Url::parse(&options.uri)
        .map_err(|e| format!("Invalid MongoDB URI '{}': {}", options.uri, e))?;
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

pub fn extract_mongo_db_name(uri: &str) -> String {
    if let Ok(parsed) = url::Url::parse(uri) {
        let seg = parsed.path().trim_matches('/');
        if let Some(first) = seg.split('/').next() {
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    "test".to_string()
}

pub async fn connect_mongo(options: DbAuthOptions) -> Result<Client, String> {
    let final_uri = build_mongo_uri(&options)?;
    let cache_key = format!(
        "{}|{}|{}",
        final_uri,
        options.auth_type.as_deref().unwrap_or(""),
        options.ssl_cert.as_deref().unwrap_or("")
    );

    if let Some(cached) = mongo_clients().get(&cache_key) {
        return Ok(cached.clone());
    }

    let mut client_options = ClientOptions::parse(&final_uri)
        .await
        .map_err(|e| format!("Invalid MongoDB URI: {}", e))?;

    if options.auth_type.as_deref() == Some("x509_cert") {
        if let Some(cert_path) = &options.ssl_cert {
            let tls_options = mongodb::options::TlsOptions::builder()
                .cert_key_file_path(std::path::PathBuf::from(cert_path))
                .build();
            client_options.tls = Some(mongodb::options::Tls::Enabled(tls_options));
        }
    }

    let client = Client::with_options(client_options)
        .map_err(|e| format!("Failed to create MongoDB client: {}", e))?;

    // Verify connectivity via ping command
    let db_name = extract_mongo_db_name(&final_uri);
    client
        .database(&db_name)
        .run_command(doc! { "ping": 1 })
        .await
        .map_err(|e| format!("MongoDB ping failed: {}", e))?;

    mongo_clients().insert(cache_key, client.clone());
    Ok(client)
}

pub async fn get_mongo_tables(options: DbAuthOptions) -> Result<Vec<String>, String> {
    let db_name = extract_mongo_db_name(&options.uri);
    let client = connect_mongo(options).await?;
    let db = client.database(&db_name);

    let mut collections = db
        .list_collection_names()
        .await
        .map_err(|e| format!("Failed to list collections: {}", e))?;
    collections.sort();
    Ok(collections)
}

fn bson_to_json(bson: &Bson) -> Value {
    match bson {
        Bson::Double(f) => serde_json::json!(f),
        Bson::String(s) => serde_json::json!(s),
        Bson::Array(arr) => Value::Array(arr.iter().map(bson_to_json).collect()),
        Bson::Document(doc) => {
            let mut map = serde_json::Map::new();
            for (k, v) in doc {
                map.insert(k.clone(), bson_to_json(v));
            }
            Value::Object(map)
        }
        Bson::Boolean(b) => serde_json::json!(b),
        Bson::Null => Value::Null,
        Bson::Int32(i) => serde_json::json!(i),
        Bson::Int64(i) => serde_json::json!(i),
        Bson::ObjectId(oid) => serde_json::json!(oid.to_hex()),
        Bson::DateTime(dt) => serde_json::json!(dt.try_to_rfc3339_string().unwrap_or_default()),
        _ => serde_json::json!(format!("{:?}", bson)),
    }
}

fn documents_to_table_data(docs: Vec<Document>, total_rows: Option<u64>) -> SqliteTableData {
    let mut columns_set = std::collections::BTreeSet::new();
    for doc in &docs {
        for key in doc.keys() {
            columns_set.insert(key.clone());
        }
    }

    let mut columns: Vec<String> = columns_set.into_iter().collect();
    if let Some(pos) = columns.iter().position(|x| x == "_id") {
        columns.remove(pos);
        columns.insert(0, "_id".to_string());
    }

    let mut table_rows = Vec::with_capacity(docs.len());
    for doc in &docs {
        let mut row_data = Vec::with_capacity(columns.len());
        for col_name in &columns {
            if let Some(val) = doc.get(col_name) {
                row_data.push(bson_to_json(val));
            } else {
                row_data.push(Value::Null);
            }
        }
        table_rows.push(row_data);
    }

    let sql_columns = columns
        .into_iter()
        .map(|name| SqliteColumn {
            is_pk: name == "_id",
            name,
            data_type: "BSON".to_string(),
        })
        .collect();

    SqliteTableData {
        columns: sql_columns,
        rows: table_rows,
        total_rows,
    }
}

pub async fn get_mongo_table_data(
    options: DbAuthOptions,
    collection: &str,
    limit: u32,
    offset: u32,
) -> Result<SqliteTableData, String> {
    let db_name = extract_mongo_db_name(&options.uri);
    let client = connect_mongo(options).await?;
    let db = client.database(&db_name);
    let coll = db.collection::<Document>(collection);

    let total_rows = coll.estimated_document_count().await.ok();

    let find_options = mongodb::options::FindOptions::builder()
        .skip(offset as u64)
        .limit(limit as i64)
        .build();

    let mut cursor = coll
        .find(Document::new())
        .with_options(find_options)
        .await
        .map_err(|e| e.to_string())?;

    let mut docs = Vec::new();
    while let Some(result) = cursor.next().await {
        docs.push(result.map_err(|e| e.to_string())?);
    }

    Ok(documents_to_table_data(docs, total_rows))
}

fn json_to_bson_doc(val: &Value, label: &str) -> Result<Document, String> {
    let bson = mongodb::bson::to_bson(val).map_err(|e| format!("Invalid {}: {}", label, e))?;
    match bson {
        Bson::Document(d) => Ok(d),
        _ => Err(format!("{} must be a JSON object", label)),
    }
}

/// Run structured JSON queries against MongoDB:
/// - `{"collection": "users", "find": {"role": "admin"}, "sort": {"createdAt": -1}, "limit": 50}`
/// - `{"collection": "users", "aggregate": [{"$match": ...}, {"$group": ...}]}`
/// - `{"collection": "users", "count": {"active": true}}`
/// - `{"collection": "users", "insertOne": {"name": "Alice"}}`
/// - `{"collection": "users", "deleteMany": {"status": "archived"}}`
pub async fn run_mongo_query(
    options: DbAuthOptions,
    query: &str,
) -> Result<SqliteQueryResult, String> {
    let started = Instant::now();
    let json_query: Value = serde_json::from_str(query).map_err(|e| {
        format!(
            "Invalid JSON query ({}). Example: {{\"collection\": \"users\", \"find\": {{}}, \"limit\": 50}}",
            e
        )
    })?;

    let obj = json_query
        .as_object()
        .ok_or_else(|| "MongoDB query must be a JSON object".to_string())?;

    let collection_name = obj
        .get("collection")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required 'collection' field in MongoDB query".to_string())?;

    let db_name = extract_mongo_db_name(&options.uri);
    let client = connect_mongo(options).await?;
    let coll = client.database(&db_name).collection::<Document>(collection_name);

    if let Some(pipeline_val) = obj.get("aggregate") {
        let arr = pipeline_val
            .as_array()
            .ok_or_else(|| "'aggregate' must be an array of pipeline stage objects".to_string())?;
        let mut pipeline = Vec::with_capacity(arr.len());
        for stage in arr {
            pipeline.push(json_to_bson_doc(stage, "pipeline stage")?);
        }
        let mut cursor = coll.aggregate(pipeline).await.map_err(|e| e.to_string())?;
        let mut docs = Vec::new();
        while let Some(res) = cursor.next().await {
            docs.push(res.map_err(|e| e.to_string())?);
        }
        let tbl = documents_to_table_data(docs, None);
        return Ok(SqliteQueryResult {
            success: true,
            columns: Some(tbl.columns),
            rows: Some(tbl.rows),
            rows_affected: None,
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
        });
    }

    if let Some(count_val) = obj.get("count") {
        let filter = json_to_bson_doc(count_val, "count filter")?;
        let count = coll.count_documents(filter).await.map_err(|e| e.to_string())?;
        return Ok(SqliteQueryResult {
            success: true,
            columns: Some(vec![SqliteColumn {
                name: "count".to_string(),
                data_type: "INTEGER".to_string(),
                is_pk: false,
            }]),
            rows: Some(vec![vec![serde_json::json!(count)]]),
            rows_affected: None,
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
        });
    }

    if let Some(insert_val) = obj.get("insertOne") {
        let doc = json_to_bson_doc(insert_val, "insertOne document")?;
        coll.insert_one(doc).await.map_err(|e| e.to_string())?;
        return Ok(SqliteQueryResult {
            success: true,
            columns: None,
            rows: None,
            rows_affected: Some(1),
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
        });
    }

    if let Some(del_val) = obj.get("deleteMany") {
        let filter = json_to_bson_doc(del_val, "deleteMany filter")?;
        let res = coll.delete_many(filter).await.map_err(|e| e.to_string())?;
        return Ok(SqliteQueryResult {
            success: true,
            columns: None,
            rows: None,
            rows_affected: Some(res.deleted_count as usize),
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
        });
    }

    let filter = match obj.get("find") {
        Some(f) => json_to_bson_doc(f, "find filter")?,
        None => Document::new(),
    };
    let limit = obj.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
    let skip = obj.get("skip").and_then(|v| v.as_u64()).unwrap_or(0);
    let sort_doc = match obj.get("sort") {
        Some(s) => Some(json_to_bson_doc(s, "sort")?),
        None => None,
    };

    let mut find_opts = mongodb::options::FindOptions::default();
    find_opts.limit = Some(limit);
    find_opts.skip = Some(skip);
    find_opts.sort = sort_doc;

    let mut cursor = coll
        .find(filter)
        .with_options(find_opts)
        .await
        .map_err(|e| e.to_string())?;
    let mut docs = Vec::new();
    while let Some(res) = cursor.next().await {
        docs.push(res.map_err(|e| e.to_string())?);
    }

    let tbl = documents_to_table_data(docs, None);
    Ok(SqliteQueryResult {
        success: true,
        columns: Some(tbl.columns),
        rows: Some(tbl.rows),
        rows_affected: None,
        execution_time_ms: Some(started.elapsed().as_millis() as u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_db_name_handles_trailing_slash_and_query() {
        assert_eq!(
            extract_mongo_db_name("mongodb://localhost:27017/mydb?retryWrites=true"),
            "mydb"
        );
        assert_eq!(extract_mongo_db_name("mongodb://localhost:27017/"), "test");
        assert_eq!(extract_mongo_db_name("mongodb://localhost:27017"), "test");
    }

    #[test]
    fn build_mongo_uri_applies_optional_credentials() {
        let opts = DbAuthOptions {
            uri: "mongodb://localhost:27017/app".to_string(),
            auth_type: None,
            username: Some("root".to_string()),
            password: Some("pass".to_string()),
            ssl_cert: None,
            ssl_key: None,
            auth_payload: None,
        };
        assert_eq!(
            build_mongo_uri(&opts).unwrap(),
            "mongodb://root:pass@localhost:27017/app"
        );
    }
}
