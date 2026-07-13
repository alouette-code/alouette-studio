use mongodb::{Client, options::ClientOptions};
use mongodb::bson::{Bson, Document};
use futures_util::stream::StreamExt;
use crate::commands::sqlite::{SqliteTableData, SqliteColumn, SqliteQueryResult};
use serde_json::Value;
use crate::commands::database::DbAuthOptions;

pub async fn connect_mongo(options: DbAuthOptions) -> Result<Client, String> {
    let mut client_options = ClientOptions::parse(&options.uri).await
        .map_err(|e| format!("Invalid MongoDB URI: {}", e))?;

    if let Some(auth_type) = &options.auth_type {
        if auth_type == "x509_cert" {
            if let Some(cert_path) = &options.ssl_cert {
                // Sử dụng rustls thông qua crate mongodb
                let tls_options = mongodb::options::TlsOptions::builder()
                    .cert_key_file_path(std::path::PathBuf::from(cert_path))
                    .build();
                client_options.tls = Some(mongodb::options::Tls::Enabled(tls_options));
            }
        }
    }

    let client = Client::with_options(client_options)
        .map_err(|e| format!("Failed to create MongoDB client: {}", e))?;
    Ok(client)
}

pub async fn get_mongo_tables(options: DbAuthOptions) -> Result<Vec<String>, String> {
    let uri = options.uri.clone();
    let client = connect_mongo(options).await?;
    // Get default database from URI, fallback to "test"
    let db_name = uri.split('/').last().unwrap_or("test").split('?').next().unwrap_or("test");
    let db = client.database(db_name);
    
    let collections = db.list_collection_names()
        .await
        .map_err(|e| format!("Failed to list collections: {}", e))?;
    
    Ok(collections)
}

fn bson_to_json(bson: &Bson) -> Value {
    match bson {
        Bson::Double(f) => serde_json::json!(f),
        Bson::String(s) => serde_json::json!(s),
        Bson::Array(arr) => {
            let json_arr: Vec<Value> = arr.iter().map(bson_to_json).collect();
            serde_json::json!(json_arr)
        },
        Bson::Document(doc) => {
            let mut map = serde_json::Map::new();
            for (k, v) in doc {
                map.insert(k.clone(), bson_to_json(v));
            }
            Value::Object(map)
        },
        Bson::Boolean(b) => serde_json::json!(b),
        Bson::Null => Value::Null,
        Bson::Int32(i) => serde_json::json!(i),
        Bson::Int64(i) => serde_json::json!(i),
        Bson::ObjectId(oid) => serde_json::json!(oid.to_hex()),
        Bson::DateTime(dt) => serde_json::json!(dt.try_to_rfc3339_string().unwrap_or_default()),
        _ => serde_json::json!(format!("{:?}", bson)),
    }
}

pub async fn get_mongo_table_data(options: DbAuthOptions, collection: &str, limit: u32, offset: u32) -> Result<SqliteTableData, String> {
    let uri = options.uri.clone();
    let client = connect_mongo(options).await?;
    let db_name = uri.split('/').last().unwrap_or("test").split('?').next().unwrap_or("test");
    let db = client.database(db_name);
    let coll = db.collection::<Document>(collection);

    let find_options = mongodb::options::FindOptions::builder()
        .skip(offset as u64)
        .limit(limit as i64)
        .build();

    let mut cursor = coll.find(mongodb::bson::Document::new()).with_options(find_options).await.map_err(|e| e.to_string())?;

    let mut columns_set = std::collections::HashSet::new();
    let mut data = Vec::new();

    while let Some(result) = cursor.next().await {
        match result {
            Ok(doc) => {
                for key in doc.keys() {
                    columns_set.insert(key.clone());
                }
                data.push(doc);
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    let mut columns: Vec<String> = columns_set.into_iter().collect();
    // Đảm bảo _id luôn ở đầu
    if let Some(pos) = columns.iter().position(|x| x == "_id") {
        columns.remove(pos);
        columns.insert(0, "_id".to_string());
    }

    let mut table_rows = Vec::new();
    for doc in data {
        let mut row_data = Vec::new();
        for col_name in &columns {
            if let Some(val) = doc.get(col_name) {
                row_data.push(bson_to_json(val));
            } else {
                row_data.push(Value::Null);
            }
        }
        table_rows.push(row_data);
    }

    let sql_columns = columns.into_iter().map(|name| SqliteColumn {
        name: name.clone(),
        data_type: "Any".to_string(),
        is_pk: name == "_id",
    }).collect();

    Ok(SqliteTableData {
        columns: sql_columns,
        rows: table_rows,
    })
}

pub async fn run_mongo_query(_options: DbAuthOptions, query: &str) -> Result<SqliteQueryResult, String> {
    // For MongoDB, we will expect a JSON string like {"collection": "users", "pipeline": [...]}
    // Or just {"find": {}}
    let _json_query: Value = serde_json::from_str(query).map_err(|e| format!("Invalid JSON query: {}", e))?;
    
    // Simplification for now: fallback to an error since we don't have a full Mongo Shell parser.
    // Real implementation would parse the JSON and use the specific MongoDB Rust Driver methods.
    Err("Run arbitrary query on MongoDB is not fully implemented yet in Rust Backend. Use the UI to explore collections.".to_string())
}
