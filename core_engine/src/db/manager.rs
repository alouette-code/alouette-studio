use std::path::Path;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::{Result, CoreError};
use crate::config::{ProjectConfig, SandboxConfig, LanguageRuntime, LanguageTool};
use crate::process::ProcessLog;
use rusqlite::params;

#[derive(Clone)]
pub struct DbManager {
    pub pool: Pool<SqliteConnectionManager>,
}

impl DbManager {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let manager = SqliteConnectionManager::file(db_path.as_ref())
            .with_init(|c| c.busy_timeout(std::time::Duration::from_millis(5000))); // Chống treo cục bộ (Retry Logic)
        let pool = Pool::builder()
            .max_size(15) // Tối ưu cho WAL mode
            .build(manager)
            .map_err(|e| CoreError::Internal(format!("Failed to build pool: {}", e)))?;
        
        Ok(DbManager { pool })
    }

    pub fn init(&self) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        
        // Cấu hình WAL cho performance cực cao
        let _: String = conn.query_row("PRAGMA journal_mode=WAL;", [], |row| row.get(0))?;
        conn.execute("PRAGMA synchronous = NORMAL;", [])?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        
        // Khởi tạo các bảng cơ bản (sẽ được tách ra repositories sau này, nhưng để an toàn thì chạy ở init)
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
        )?;

        let _ = conn.execute("ALTER TABLE projects ADD COLUMN max_log_lines INTEGER;", []);

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
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_logs_project_timestamp ON logs(project_id, timestamp);",
            [],
        )?;

        // Các bảng khác...
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sandbox_configs (
                project_id TEXT PRIMARY KEY,
                config TEXT NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS language_runtimes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                install_command TEXT NOT NULL,
                versions TEXT NOT NULL DEFAULT '[]',
                tools TEXT NOT NULL DEFAULT '[]'
            );",
            [],
        )?;

        Ok(())
    }
    pub fn save_project(&self, config: &ProjectConfig) -> Result<()> {
        let repo = crate::db::repositories::project_repo::ProjectRepository::new(self.pool.clone());
        crate::db::traits::IProjectRepository::save_project(&repo, config)
    }

    pub fn delete_project(&self, project_id: &str) -> Result<()> {
        let repo = crate::db::repositories::project_repo::ProjectRepository::new(self.pool.clone());
        crate::db::traits::IProjectRepository::delete_project(&repo, project_id)
    }

    pub fn load_projects(&self) -> Result<Vec<ProjectConfig>> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT id, name, command, args, cwd, setup_command, setup_args, auto_restart, env, max_cpu_percent, max_ram_mb, port, source, terminal_mode, toolchain, toolchain_version, enable_tunnel, max_log_lines FROM projects")?;
        let project_iter = stmt.query_map([], |row| {
            let args_json: String = row.get(3)?;
            let args: Vec<String> = serde_json::from_str(&args_json).unwrap_or_default();
            
            let setup_args_json: Option<String> = row.get(6)?;
            let setup_args = setup_args_json.map(|s| serde_json::from_str(&s).unwrap_or_default());
            
            let env_json: Option<String> = row.get(8)?;
            let env = env_json.map(|s| serde_json::from_str(&s).unwrap_or_default());

            let auto_restart_int: Option<i32> = row.get(7)?;
            let auto_restart = auto_restart_int.map(|v| v != 0);
            
            let enable_tunnel_int: Option<i32> = row.get(16)?;
            let enable_tunnel = enable_tunnel_int.map(|v| v != 0);

            let max_ram_mb: Option<i64> = row.get(10)?;

            Ok(ProjectConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                command: row.get(2)?,
                args,
                cwd: row.get(4)?,
                setup_command: row.get(5)?,
                setup_args,
                auto_restart,
                env,
                max_cpu_percent: row.get(9)?,
                max_ram_mb: max_ram_mb.map(|v| v as u64),
                port: row.get(11)?,
                source: row.get(12)?,
                terminal_mode: row.get(13)?,
                toolchain: row.get(14)?,
                toolchain_version: row.get(15)?,
                enable_tunnel,
                max_log_lines: row.get(17)?,
            })
        })?;

        let mut projects = Vec::new();
        for p in project_iter {
            projects.push(p?);
        }
        Ok(projects)
    }

    pub fn insert_log(&self, project_id: &str, stream: &str, text: &str, timestamp: u64) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO logs (project_id, stream, text, timestamp) VALUES (?1, ?2, ?3, ?4);",
            params![project_id, stream, text, timestamp as i64],
        )?;
        Ok(())
    }

    pub fn get_project_max_log_lines(&self, project_id: &str) -> Result<Option<usize>> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        let res: Option<i64> = conn.query_row(
            "SELECT max_log_lines FROM projects WHERE id = ?1;",
            params![project_id],
            |row| row.get(0),
        ).unwrap_or(None);
        Ok(res.map(|v| v as usize))
    }

    pub fn prune_logs(&self, project_id: &str, limit: usize) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM logs WHERE project_id = ?1;",
            params![project_id],
            |row| row.get(0),
        )?;
        
        let limit_i64 = limit as i64;
        if count > limit_i64 {
            let to_delete = count - limit_i64;
            conn.execute(
                "DELETE FROM logs WHERE id IN (SELECT id FROM logs WHERE project_id = ?1 ORDER BY timestamp ASC LIMIT ?2);",
                params![project_id, to_delete],
            )?;
        }
        Ok(())
    }

    // Sandbox Config CRUD

    pub fn save_sandbox_config(&self, config: &SandboxConfig) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        let config_json = serde_json::to_string(config).map_err(|e| CoreError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO sandbox_configs (project_id, config) VALUES (?1, ?2);",
            params![config.project_id, config_json],
        )?;
        Ok(())
    }

    pub fn load_all_sandbox_configs(&self) -> Result<Vec<SandboxConfig>> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT config FROM sandbox_configs;")?;
        let rows = stmt.query_map([], |row| {
            let config_json: String = row.get(0)?;
            Ok(config_json)
        })?;
        let mut configs = Vec::new();
        for row in rows {
            let json = row?;
            let config: SandboxConfig = serde_json::from_str(&json).map_err(|e| CoreError::Internal(e.to_string()))?;
            configs.push(config);
        }
        Ok(configs)
    }

    pub fn delete_sandbox_config(&self, project_id: &str) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM sandbox_configs WHERE project_id = ?1;", params![project_id])?;
        Ok(())
    }

    // Language Runtime CRUD

    pub fn save_language_runtime(&self, runtime: &LanguageRuntime) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        let versions_json = serde_json::to_string(&runtime.versions).map_err(|e| CoreError::Internal(e.to_string()))?;
        let tools_json = serde_json::to_string(&runtime.tools).map_err(|e| CoreError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO language_runtimes (id, name, install_command, versions, tools) VALUES (?1, ?2, ?3, ?4, ?5);",
            params![runtime.id, runtime.name, runtime.install_command, versions_json, tools_json],
        )?;
        Ok(())
    }

    pub fn load_all_language_runtimes(&self) -> Result<Vec<LanguageRuntime>> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT id, name, install_command, versions, tools FROM language_runtimes;")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let install_command: String = row.get(2)?;
            let versions_json: String = row.get(3)?;
            let tools_json: String = row.get(4)?;
            Ok((id, name, install_command, versions_json, tools_json))
        })?;
        let mut runtimes = Vec::new();
        for row in rows {
            let (id, name, install_command, versions_json, tools_json) = row?;
            let versions: Vec<String> = serde_json::from_str(&versions_json).map_err(|e| CoreError::Internal(e.to_string()))?;
            let tools: Vec<LanguageTool> = serde_json::from_str(&tools_json).map_err(|e| CoreError::Internal(e.to_string()))?;
            runtimes.push(LanguageRuntime { id, name, install_command, versions, tools });
        }
        Ok(runtimes)
    }

    pub fn delete_language_runtime(&self, runtime_id: &str) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM language_runtimes WHERE id = ?1;", params![runtime_id])?;
        Ok(())
    }

    pub fn get_logs(&self, project_id: &str, limit: usize) -> Result<Vec<ProcessLog>> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT project_id, stream, text, timestamp FROM logs WHERE project_id = ?1 ORDER BY timestamp DESC, id DESC LIMIT ?2;")?;
        let log_iter = stmt.query_map(params![project_id, limit as i64], |row| {
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
        })?;
        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log?);
        }
        logs.reverse();
        Ok(logs)
    }
}
