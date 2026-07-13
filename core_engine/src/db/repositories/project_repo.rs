use crate::error::{CoreError, Result};
use crate::config::ProjectConfig;
use rusqlite::params;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::db::traits::IProjectRepository;

pub struct ProjectRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl ProjectRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }
}

impl IProjectRepository for ProjectRepository {
    fn save_project(&self, config: &ProjectConfig) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        
        let args_json = serde_json::to_string(&config.args)?;
        let setup_args_json = config.setup_args.as_ref().map(|sa| serde_json::to_string(sa)).transpose()?;
        let env_json = config.env.as_ref().map(|env| serde_json::to_string(env)).transpose()?;
        
        let auto_restart_int = config.auto_restart.map(|b| if b { 1 } else { 0 });
        let enable_tunnel_int = config.enable_tunnel.map(|b| if b { 1 } else { 0 });

        conn.execute(
            "INSERT OR REPLACE INTO projects (
                id, name, command, args, cwd, setup_command, setup_args,
                auto_restart, env, max_cpu_percent, max_ram_mb, port,
                source, terminal_mode, toolchain, toolchain_version, enable_tunnel, max_log_lines
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18);",
            params![
                config.id, config.name, config.command, args_json, config.cwd,
                config.setup_command, setup_args_json, auto_restart_int, env_json,
                config.max_cpu_percent, config.max_ram_mb.map(|v| v as i64), config.port,
                config.source, config.terminal_mode, config.toolchain, config.toolchain_version,
                enable_tunnel_int, config.max_log_lines
            ],
        )?;

        Ok(())
    }

    fn delete_project(&self, project_id: &str) -> Result<()> {
        let conn = self.pool.get().map_err(|e| CoreError::Internal(e.to_string()))?;
        conn.execute("DELETE FROM projects WHERE id = ?1;", params![project_id])?;
        Ok(())
    }
}
