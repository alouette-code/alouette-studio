import re

with open("db.rs", "r") as f:
    content = f.read()

# Replace DbManager struct
content = re.sub(
    r'pub struct DbManager \{.*?\}',
    'pub struct DbManager {\n    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,\n}',
    content,
    flags=re.DOTALL
)

# Replace new()
new_func = """pub fn new<P: AsRef<Path>>(db_path: P) -> crate::error::Result<Self> {
        let manager = r2d2_sqlite::SqliteConnectionManager::file(db_path.as_ref());
        let pool = r2d2::Pool::new(manager).map_err(|e| crate::error::CoreError::Internal(e.to_string()))?;
        Ok(DbManager { pool })
    }"""
content = re.sub(
    r'pub fn new<P: AsRef<Path>>\(db_path: P\) -> Self \{.*?\}',
    new_func,
    content,
    flags=re.DOTALL
)

# Replace Result<T, String> with crate::error::Result<T>
content = content.replace("Result<(), String>", "crate::error::Result<()>")
content = content.replace("Result<Vec<ProjectConfig>, String>", "crate::error::Result<Vec<ProjectConfig>>")
content = content.replace("Result<Option<u32>, String>", "crate::error::Result<Option<u32>>")
content = content.replace("Result<Option<SandboxConfig>, String>", "crate::error::Result<Option<SandboxConfig>>")
content = content.replace("Result<Vec<SandboxConfig>, String>", "crate::error::Result<Vec<SandboxConfig>>")
content = content.replace("Result<Vec<LanguageRuntime>, String>", "crate::error::Result<Vec<LanguageRuntime>>")
content = content.replace("Result<Vec<ProcessLog>, String>", "crate::error::Result<Vec<ProcessLog>>")

# Replace Connection::open with pool.get()
content = re.sub(
    r'let (mut )?conn = Connection::open\(&self\.db_path\)\s*\.map_err\(\|e\| format!\("Failed to open database: \{\}", e\)\)\?;',
    r'let \1conn = self.pool.get().map_err(|e| crate::error::CoreError::Internal(format!("Failed to get DB connection: {}", e)))?;',
    content
)

# Replace map_err with ?, mapping when necessary
content = re.sub(
    r'\.map_err\(\|e\| format!\("Failed to serialize args: \{\}", e\)\)\?',
    r'?',
    content
)
content = re.sub(
    r'\.map_err\(\|e\| format!\("Failed to serialize setup_args: \{\}", e\)\)\?',
    r'?',
    content
)
content = re.sub(
    r'\.map_err\(\|e\| format!\("Failed to serialize env: \{\}", e\)\)\?',
    r'?',
    content
)
content = re.sub(
    r'\.map_err\(\|e\| format!\(".*?: \{\}", e\)\)\?',
    r'?',
    content
)
with open("db_new.rs", "w") as f:
    f.write(content)

