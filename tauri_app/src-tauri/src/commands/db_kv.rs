#![allow(dead_code)]
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct KvPair {
    pub key: String,
    pub value: String,
}

pub async fn get_kv_namespaces(uri: &str) -> Result<Vec<String>, String> {
    // Determine the KV engine based on prefix
    if uri.starts_with("sled://") {
        let path = uri.trim_start_matches("sled://");
        let db = sled::open(path).map_err(|e| e.to_string())?;
        let tree_names = db.tree_names()
            .into_iter()
            .map(|name| String::from_utf8_lossy(&name).into_owned())
            .collect();
        Ok(tree_names)
    } else if uri.starts_with("redb://") {
        // Redb requires opening tables specifically, here we just return a default namespace for now
        Ok(vec!["default".to_string()])
    } else {
        Err(format!("KV Engine not supported or uri malformed: {}", uri))
    }
}

pub async fn get_kv_data(uri: &str, namespace: &str) -> Result<Vec<KvPair>, String> {
    if uri.starts_with("sled://") {
        let path = uri.trim_start_matches("sled://");
        let db = sled::open(path).map_err(|e| e.to_string())?;
        let tree = db.open_tree(namespace).map_err(|e| e.to_string())?;
        
        let mut results = Vec::new();
        for item in tree.iter() {
            let (k, v) = item.map_err(|e| e.to_string())?;
            results.push(KvPair {
                key: String::from_utf8_lossy(&k).into_owned(),
                value: String::from_utf8_lossy(&v).into_owned(),
            });
            if results.len() > 100 { break; } // Limit to 100 for now
        }
        Ok(results)
    } else {
        Err(format!("Fetching data for this KV engine is not implemented yet."))
    }
}
