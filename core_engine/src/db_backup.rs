use std::path::{Path, PathBuf};
use rusqlite::{params, Connection};
use crate::config::{ProjectConfig, SandboxConfig, LanguageRuntime, LanguageTool};
use crate::process::ProcessLog;

/// A high-performance, thread-safe manager for the SQLite persistence layer.
/// Handles SQLite operations inside dedicated or on-demand connection contexts,
/// optimized for multi-threaded access using WAL journal mode.
#[derive(Debug, Clone)]
pub struct DbManager {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl DbManager {
    /// Instantiates a new database manager targeting the specified path.
    pub fn new<P: AsRef<Path>>(db_path: P) -> crate::error::Result<Self> {
        let manager = r2d2_sqlite::SqliteConnectionManager::file(db_path.as_ref());
        let pool = r2d2::Pool::new(manager).map_err(|e| crate::error::CoreError::Internal(e.to_string()))?;
        Ok(DbManager { pool })
    }

    /// Initializes tables, sets WAL mode for concurrent execution safety,
    /// and establishes standard indices.
    pub fn init(&self) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        // Enable Write-Ahead Logging (WAL) for safe concurrent reads/writes
        let _: String = conn
            .query_row("PRAGMA journal_mode=WAL;", [], |row| row.get(0))
            ?;

        // Enable foreign key cascading constraints
        conn.execute("PRAGMA foreign_keys = ON;", [])
            ?;

        // 1. Create Server Configurations Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT NOT NULL,
                cwd TEXT,
                setup_command TEXT,
                setup_args TEXT,
                auto_restart INTEGER,
                env TEXT,
                max_cpu_percent INTEGER,
                max_ram_mb INTEGER,
                port INTEGER,
                source TEXT,
                terminal_mode TEXT,
                toolchain TEXT,
                toolchain_version TEXT,
                enable_tunnel INTEGER,
                max_log_lines INTEGER
            );",
            [],
        )
        ?;

        // Dynamic column migration for backward compatibility
        let _ = conn.execute("ALTER TABLE projects ADD COLUMN max_log_lines INTEGER;", []);

        // 2. Create Process/Server Execution Logs Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id TEXT NOT NULL,
                stream TEXT NOT NULL,
                text TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );",
            [],
        )
        ?;

        // Create composite index for ultra-fast historical log queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_logs_project_timestamp ON logs(project_id, timestamp);",
            [],
        )
        ?;

        // 3. Create Sandbox Configurations Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sandbox_configs (
                project_id TEXT PRIMARY KEY,
                config TEXT NOT NULL
            );",
            [],
        )
        ?;

        // 4. Create Language Runtimes Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS language_runtimes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                install_command TEXT NOT NULL,
                versions TEXT NOT NULL DEFAULT '[]',
                tools TEXT NOT NULL DEFAULT '[]'
            );",
            [],
        )
        ?;

        Ok(())
    }

    /// Persists or updates a server/project configuration in SQLite.
    pub fn save_project(&self, config: &ProjectConfig) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let args_json = serde_json::to_string(&config.args)
            ?;

        let setup_args_json = config
            .setup_args
            .as_ref()
            .map(|sa| serde_json::to_string(sa))
            .transpose()
            ?;

        let env_json = config
            .env
            .as_ref()
            .map(|env| serde_json::to_string(env))
            .transpose()
            ?;

        let auto_restart_int = config.auto_restart.map(|b| if b { 1 } else { 0 });
        let enable_tunnel_int = config.enable_tunnel.map(|b| if b { 1 } else { 0 });

        conn.execute(
            "INSERT OR REPLACE INTO projects (
                id, name, command, args, cwd, setup_command, setup_args,
                auto_restart, env, max_cpu_percent, max_ram_mb, port,
                source, terminal_mode, toolchain, toolchain_version, enable_tunnel, max_log_lines
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18);",
            params![
                config.id,
                config.name,
                config.command,
                args_json,
                config.cwd,
                config.setup_command,
                setup_args_json,
                auto_restart_int,
                env_json,
                config.max_cpu_percent,
                config.max_ram_mb.map(|v| v as i64),
                config.port,
                config.source,
                config.terminal_mode,
                config.toolchain,
                config.toolchain_version,
                enable_tunnel_int,
                config.max_log_lines
            ],
        )
        ?;

        Ok(())
    }

    /// Deregisters a server/project configuration, cascading deletion to all related logs.
    pub fn delete_project(&self, project_id: &str) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        conn.execute("PRAGMA foreign_keys = ON;", [])
            ?;

        conn.execute("DELETE FROM projects WHERE id = ?1;", params![project_id])
            ?;

        Ok(())
    }

    /// Loads all saved server/project configurations from SQLite.
    pub fn load_projects(&self) -> crate::error::Result<Vec<ProjectConfig>> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, command, args, cwd, setup_command, setup_args,
                        auto_restart, env, max_cpu_percent, max_ram_mb, port,
                        source, terminal_mode, toolchain, toolchain_version, enable_tunnel, max_log_lines
                 FROM projects;",
            )
            ?;

        let project_iter = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let command: String = row.get(2)?;
                let args_raw: String = row.get(3)?;
                let cwd: Option<String> = row.get(4)?;
                let setup_command: Option<String> = row.get(5)?;
                let setup_args_raw: Option<String> = row.get(6)?;
                let auto_restart_int: Option<i32> = row.get(7)?;
                let env_raw: Option<String> = row.get(8)?;
                let max_cpu_percent: Option<u32> = row.get(9)?;
                let max_ram_mb_i64: Option<i64> = row.get(10)?;
                let max_ram_mb = max_ram_mb_i64.map(|v| v as u64);
                let port: Option<u16> = row.get(11)?;
                let source: Option<String> = row.get(12)?;
                let terminal_mode: Option<String> = row.get(13)?;
                let toolchain: Option<String> = row.get(14)?;
                let toolchain_version: Option<String> = row.get(15)?;
                let enable_tunnel_int: Option<i32> = row.get(16)?;
                let max_log_lines: Option<u32> = row.get(17)?;

                let args: Vec<String> = serde_json::from_str(&args_raw).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let setup_args: Option<Vec<String>> = setup_args_raw
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            6,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let env: Option<std::collections::HashMap<String, String>> = env_raw
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            8,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let auto_restart = auto_restart_int.map(|i| i != 0);
                let enable_tunnel = enable_tunnel_int.map(|i| i != 0);

                Ok(ProjectConfig {
                    id,
                    name,
                    command,
                    args,
                    cwd,
                    setup_command,
                    setup_args,
                    auto_restart,
                    env,
                    max_cpu_percent,
                    max_ram_mb,
                    port,
                    source,
                    terminal_mode,
                    toolchain,
                    toolchain_version,
                    enable_tunnel,
                    max_log_lines,
                })
            })
            ?;

        let mut projects = Vec::new();
        for proj in project_iter {
            projects.push(proj?);
        }

        Ok(projects)
    }

    /// Inserts a new execution log line into SQLite.
    pub fn insert_log(
        &self,
        project_id: &str,
        stream: &str,
        text: &str,
        timestamp: u64,
    ) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        conn.execute(
            "INSERT INTO logs (project_id, stream, text, timestamp) VALUES (?1, ?2, ?3, ?4);",
            params![project_id, stream, text, timestamp as i64],
        )
        ?;

        Ok(())
    }

    /// Fetches the configured max log lines retention limit for a project.
    pub fn get_project_max_log_lines(&self, project_id: &str) -> crate::error::Result<Option<u32>> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT max_log_lines FROM projects WHERE id = ?1;")
            ?;

        let mut rows = stmt
            .query(params![project_id])
            ?;

        if let Some(row) = rows.next()? {
            let max_lines: Option<u32> = row.get(0)?;
            Ok(max_lines)
        } else {
            Ok(None)
        }
    }

    /// Prunes database logs, preserving only the most recent N lines for a specific project.
    pub fn prune_logs(&self, project_id: &str, keep_limit: usize) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        conn.execute(
            "DELETE FROM logs
             WHERE project_id = ?1
               AND id NOT IN (
                   SELECT id FROM logs
                   WHERE project_id = ?1
                   ORDER BY timestamp DESC, id DESC
                   LIMIT ?2
               );",
            params![project_id, keep_limit as i64],
        )
        ?;

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════
    // Sandbox Config CRUD
    // ═══════════════════════════════════════════════════════════════

    /// Save a sandbox configuration for a project.
    pub fn save_sandbox_config(&self, config: &SandboxConfig) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let config_json = serde_json::to_string(config)
            ?;

        conn.execute(
            "INSERT OR REPLACE INTO sandbox_configs (project_id, config) VALUES (?1, ?2);",
            params![config.project_id, config_json],
        )
        ?;

        Ok(())
    }

    /// Load a sandbox config for a specific project.
    pub fn load_sandbox_config(&self, project_id: &str) -> crate::error::Result<Option<SandboxConfig>> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT config FROM sandbox_configs WHERE project_id = ?1;")
            ?;

        let mut rows = stmt
            .query(params![project_id])
            ?;

        if let Some(row) = rows.next()? {
            let config_json: String = row.get(0)?;
            let config: SandboxConfig = serde_json::from_str(&config_json)
                ?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// Load all sandbox configs.
    pub fn load_all_sandbox_configs(&self) -> crate::error::Result<Vec<SandboxConfig>> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT config FROM sandbox_configs;")
            ?;

        let rows = stmt
            .query_map([], |row| {
                let config_json: String = row.get(0)?;
                Ok(config_json)
            })
            ?;

        let mut configs = Vec::new();
        for row in rows {
            let json = row?;
            let config: SandboxConfig = serde_json::from_str(&json)
                ?;
            configs.push(config);
        }

        Ok(configs)
    }

    /// Delete a sandbox configuration for a project.
    pub fn delete_sandbox_config(&self, project_id: &str) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        conn.execute(
            "DELETE FROM sandbox_configs WHERE project_id = ?1;",
            params![project_id],
        )
        ?;

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════
    // Language Runtime CRUD
    // ═══════════════════════════════════════════════════════════════

    pub fn save_language_runtime(&self, runtime: &LanguageRuntime) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let versions_json = serde_json::to_string(&runtime.versions)
            ?;
        let tools_json = serde_json::to_string(&runtime.tools)
            ?;

        conn.execute(
            "INSERT OR REPLACE INTO language_runtimes (id, name, install_command, versions, tools) VALUES (?1, ?2, ?3, ?4, ?5);",
            rusqlite::params![runtime.id, runtime.name, runtime.install_command, versions_json, tools_json],
        )
        ?;

        Ok(())
    }

    pub fn load_all_language_runtimes(&self) -> crate::error::Result<Vec<LanguageRuntime>> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT id, name, install_command, versions, tools FROM language_runtimes;")
            ?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let install_command: String = row.get(2)?;
                let versions_json: String = row.get(3)?;
                let tools_json: String = row.get(4)?;
                Ok((id, name, install_command, versions_json, tools_json))
            })
            ?;

        let mut runtimes = Vec::new();
        for row in rows {
            let (id, name, install_command, versions_json, tools_json) = row
                ?;
            let versions: Vec<String> = serde_json::from_str(&versions_json)
                ?;
            let tools: Vec<LanguageTool> = serde_json::from_str(&tools_json)
                ?;
            runtimes.push(LanguageRuntime { id, name, install_command, versions, tools });
        }
        Ok(runtimes)
    }

    pub fn delete_language_runtime(&self, runtime_id: &str) -> crate::error::Result<()> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;
        conn.execute(
            "DELETE FROM language_runtimes WHERE id = ?1;",
            rusqlite::params![runtime_id],
        )
        ?;
        Ok(())
    }

    /// Retrieves the last N logs chronologically for a project.
    pub fn get_logs(&self, project_id: &str, limit: usize) -> crate::error::Result<Vec<ProcessLog>> {
        let conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT project_id, stream, text, timestamp
                 FROM logs
                 WHERE project_id = ?1
                 ORDER BY timestamp DESC, id DESC
                 LIMIT ?2;",
            )
            ?;

        let log_iter = stmt
            .query_map(params![project_id, limit as i64], |row| {
                let project_id: String = row.get(0)?;
                let stream: String = row.get(1)?;
                let text: String = row.get(2)?;
                let timestamp_i64: i64 = row.get(3)?;

                Ok(ProcessLog {
                    project_id,
                    stream,
                    text,
                    timestamp: timestamp_i64 as u64,
                })
            })
            ?;

        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log?);
        }

        // Since we queried with DESC to get latest, reverse it to return chronologically
        logs.reverse();

        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sqlite_persistence_flow() {
        let temp_dir = std::env::temp_dir();
        let db_file = temp_dir.join("alouette_test_db.db");
        let _ = fs::remove_file(&db_file); // Clean up if any left

        let db = DbManager::new(&db_file);
        assert!(db.init().is_ok());

        // Test configuration insertion
        let project = ProjectConfig {
            id: "test-db-id".to_string(),
            name: "Test DB Project".to_string(),
            command: "echo".to_string(),
            args: vec!["hello".to_string(), "world".to_string()],
            cwd: Some("/test/path".to_string()),
            setup_command: None,
            setup_args: None,
            auto_restart: Some(true),
            env: Some({
                let mut map = std::collections::HashMap::new();
                map.insert("KEY".to_string(), "VAL".to_string());
                map
            }),
            max_cpu_percent: Some(90),
            max_ram_mb: Some(1024),
            port: Some(8080),
            source: None,
            terminal_mode: None,
            toolchain: None,
            toolchain_version: None,
            enable_tunnel: None,
            max_log_lines: Some(100),
        };

        assert!(db.save_project(&project).is_ok());

        // Load & verify configurations
        let loaded = db.load_projects().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "test-db-id");
        assert_eq!(loaded[0].name, "Test DB Project");
        assert_eq!(loaded[0].args, vec!["hello".to_string(), "world".to_string()]);
        assert_eq!(loaded[0].port, Some(8080));
        assert_eq!(loaded[0].auto_restart, Some(true));
        assert_eq!(loaded[0].max_log_lines, Some(100));

        // Verify get_project_max_log_lines helper
        assert_eq!(db.get_project_max_log_lines("test-db-id").unwrap(), Some(100));
        assert_eq!(db.get_project_max_log_lines("non-existent-id").unwrap(), None);

        // Test log insertion & retrieval
        assert!(db.insert_log("test-db-id", "stdout", "log line 1", 1000).is_ok());
        assert!(db.insert_log("test-db-id", "stderr", "log line 2", 2000).is_ok());
        assert!(db.insert_log("test-db-id", "stdout", "log line 3", 3000).is_ok());

        let logs = db.get_logs("test-db-id", 2).unwrap();
        assert_eq!(logs.len(), 2);
        // Reversed chronologically
        assert_eq!(logs[0].text, "log line 2");
        assert_eq!(logs[1].text, "log line 3");

        // Test pruning (keep only 2 logs)
        assert!(db.prune_logs("test-db-id", 2).is_ok());
        let logs_after_prune = db.get_logs("test-db-id", 10).unwrap();
        assert_eq!(logs_after_prune.len(), 2);
        assert_eq!(logs_after_prune[0].text, "log line 2");
        assert_eq!(logs_after_prune[1].text, "log line 3");

        // Test deletion cascading
        assert!(db.delete_project("test-db-id").is_ok());
        let loaded_empty = db.load_projects().unwrap();
        assert_eq!(loaded_empty.len(), 0);

        // Logs should be deleted as well due to cascading
        let logs_empty = db.get_logs("test-db-id", 10).unwrap();
        assert_eq!(logs_empty.len(), 0);

        let _ = fs::remove_file(&db_file);
    }
}
