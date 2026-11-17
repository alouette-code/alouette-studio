import { useState, useEffect, useMemo } from "react";
import {
  Database,
  Plus,
  Trash2,
  RefreshCw,
  AlertCircle,
  Check,
  Key,
  FileSpreadsheet,
  HelpCircle,
  Columns,
  TerminalSquare,
  Play,
  ChevronLeft,
  ChevronRight,
  Download,
  Search,
  ArrowUpDown,
  Maximize2,
  X,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import Editor from "@monaco-editor/react";
import AlouetteSelect from "./AlouetteSelect";
import "./SqliteEditor.css";

interface SqliteColumn {
  name: string;
  data_type: string;
  is_pk: boolean;
}

interface SqliteTableData {
  columns: SqliteColumn[];
  rows: any[][];
  total_rows?: number | null;
}

interface SqliteEditorProps {
  filePath: string;
  triggerConfirm?: (message: string, onConfirm: () => void) => void;
  triggerToast?: (message: string, type: "success" | "error" | "info") => void;
}

function detectEngineInfo(uri: string): {
  label: string;
  queryLang: "sql" | "json" | "plaintext";
  supportsSchemaAlter: boolean;
  defaultQuery: string;
} {
  if (uri.startsWith("postgres://") || uri.startsWith("postgresql://")) {
    return {
      label: "PostgreSQL",
      queryLang: "sql",
      supportsSchemaAlter: true,
      defaultQuery: "SELECT NOW(), version();",
    };
  }
  if (uri.startsWith("mysql://")) {
    return {
      label: "MySQL",
      queryLang: "sql",
      supportsSchemaAlter: true,
      defaultQuery: "SHOW TABLES;",
    };
  }
  if (uri.startsWith("duckdb://")) {
    return {
      label: "DuckDB",
      queryLang: "sql",
      supportsSchemaAlter: true,
      defaultQuery: "SHOW ALL TABLES;",
    };
  }
  if (uri.startsWith("mongodb://") || uri.startsWith("mongodb+srv://")) {
    return {
      label: "MongoDB",
      queryLang: "json",
      supportsSchemaAlter: false,
      defaultQuery: '{\n  "collection": "users",\n  "find": {},\n  "limit": 50\n}',
    };
  }
  if (uri.startsWith("redis://") || uri.startsWith("rediss://")) {
    return {
      label: "Redis",
      queryLang: "plaintext",
      supportsSchemaAlter: false,
      defaultQuery: "INFO server",
    };
  }
  if (uri.startsWith("sled://")) {
    return {
      label: "Sled KV",
      queryLang: "plaintext",
      supportsSchemaAlter: false,
      defaultQuery: "GET __sled__default key",
    };
  }
  if (uri.startsWith("redb://")) {
    return {
      label: "Redb KV",
      queryLang: "plaintext",
      supportsSchemaAlter: false,
      defaultQuery: "",
    };
  }
  return {
    label: "SQLite DB",
    queryLang: "sql",
    supportsSchemaAlter: true,
    defaultQuery: "SELECT name, sql FROM sqlite_master WHERE type='table';",
  };
}

function formatCellDisplay(val: any): string {
  if (val === null || val === undefined) return "NULL";
  if (typeof val === "object") return JSON.stringify(val);
  return String(val);
}

function triggerFileDownload(filename: string, content: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export default function SqliteEditor({
  filePath,
  triggerConfirm,
  triggerToast,
}: SqliteEditorProps) {
  const engine = useMemo(() => detectEngineInfo(filePath), [filePath]);

  const [tables, setTables] = useState<string[]>([]);
  const [activeTable, setActiveTable] = useState<string | null>(null);
  const [data, setData] = useState<SqliteTableData | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Filter / Sort / Inspect
  const [searchFilter, setSearchFilter] = useState<string>("");
  const [sortColIdx, setSortColIdx] = useState<number | null>(null);
  const [sortAsc, setSortAsc] = useState<boolean>(true);
  const [inspectedCell, setInspectedCell] = useState<{
    colName: string;
    value: any;
  } | null>(null);

  // Cell editing state
  const [editingCell, setEditingCell] = useState<{
    rowIndex: number;
    colName: string;
  } | null>(null);
  const [editValue, setEditValue] = useState<string>("");

  // Add column form state
  const [showAddColumn, setShowAddColumn] = useState<boolean>(false);
  const [newColName, setNewColName] = useState<string>("");
  const [newColType, setNewColType] = useState<string>("TEXT");

  // Save status toast
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "success" | "error">("idle");
  const [statusMessage, setStatusMessage] = useState<string>("");

  // View mode
  const [viewMode, setViewMode] = useState<"table" | "query">("table");

  // Raw Query state
  const [rawQuery, setRawQuery] = useState<string>(engine.defaultQuery);
  const [queryResult, setQueryResult] = useState<{
    success: boolean;
    columns: SqliteColumn[] | null;
    rows: any[][] | null;
    rows_affected: number | null;
    execution_time_ms?: number | null;
    error?: string;
  } | null>(null);
  const [isQueryRunning, setIsQueryRunning] = useState<boolean>(false);
  const [limit, setLimit] = useState(100);
  const [offset, setOffset] = useState(0);

  const fileName = useMemo(() => {
    try {
      if (filePath.includes("://")) {
        const stripped = filePath.split("://")[1] || filePath;
        const withoutAuth = stripped.includes("@") ? stripped.split("@")[1] : stripped;
        return withoutAuth.split(/[\\/]/).filter(Boolean).pop() || withoutAuth;
      }
    } catch {}
    return filePath.split(/[\\/]/).pop() || filePath;
  }, [filePath]);

  const loadTables = async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<string[]>("get_db_tables", {
        options: { uri: filePath, auth_type: "basic" },
      });
      setTables(res);
      if (res.length > 0) {
        if (!activeTable || !res.includes(activeTable)) {
          setOffset(0);
          setActiveTable(res[0]);
        }
      } else {
        setActiveTable(null);
        setData(null);
      }
    } catch (err: any) {
      console.error("Error loading tables:", err);
      setError(`Failed to read database: ${err.toString()}`);
    } finally {
      setLoading(false);
    }
  };

  const loadTableData = async (tableName: string, currentOffset = offset, currentLimit = limit) => {
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<SqliteTableData>("get_db_table_data", {
        options: { uri: filePath, auth_type: "basic" },
        table: tableName,
        limit: currentLimit,
        offset: currentOffset,
      });
      setData(res);
    } catch (err: any) {
      console.error("Error loading table data:", err);
      setError(`Failed to load table data: ${err.toString()}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    setRawQuery(engine.defaultQuery);
    setOffset(0);
    setSortColIdx(null);
    setSearchFilter("");
    loadTables();
  }, [filePath]);

  useEffect(() => {
    if (activeTable) {
      loadTableData(activeTable, offset, limit);
    } else {
      setData(null);
    }
    setShowAddColumn(false);
  }, [activeTable, filePath, limit, offset]);

  const handleSelectTable = (tbl: string) => {
    setOffset(0);
    setSortColIdx(null);
    setSearchFilter("");
    setActiveTable(tbl);
  };

  const triggerStatus = (status: "saving" | "success" | "error", message: string) => {
    setSaveStatus(status);
    setStatusMessage(message);
    if (status !== "saving") {
      setTimeout(() => setSaveStatus("idle"), 3000);
    }
  };

  const pkColumn = data?.columns.find((c) => c.is_pk);
  const pkIndex = data?.columns.findIndex((c) => c.is_pk) ?? -1;

  // Filtered + sorted rows keeping original row index for safe PK edits
  const visibleRows = useMemo(() => {
    if (!data) return [];
    const term = searchFilter.trim().toLowerCase();
    let indexed = data.rows.map((row, idx) => ({ row, origIdx: idx }));

    if (term) {
      indexed = indexed.filter(({ row }) =>
        row.some((val) => formatCellDisplay(val).toLowerCase().includes(term)),
      );
    }

    if (sortColIdx !== null) {
      indexed.sort((a, b) => {
        const av = a.row[sortColIdx];
        const bv = b.row[sortColIdx];
        if (av === null && bv === null) return 0;
        if (av === null) return sortAsc ? -1 : 1;
        if (bv === null) return sortAsc ? 1 : -1;
        if (typeof av === "number" && typeof bv === "number") {
          return sortAsc ? av - bv : bv - av;
        }
        return sortAsc
          ? String(av).localeCompare(String(bv))
          : String(bv).localeCompare(String(av));
      });
    }
    return indexed;
  }, [data, searchFilter, sortColIdx, sortAsc]);

  const handleCellSave = async (origRowIndex: number, colName: string, originalValue: any) => {
    setEditingCell(null);
    if (!activeTable || !data || !pkColumn || pkIndex === -1) return;

    const row = data.rows[origRowIndex];
    const pkValue = row[pkIndex];

    const colInfo = data.columns.find((c) => c.name === colName);
    let formattedValue: any = editValue;

    if (editValue === "" || editValue.toLowerCase() === "null") {
      formattedValue = null;
    } else if (colInfo) {
      const type = colInfo.data_type.toUpperCase();
      if (type.includes("BOOL")) {
        formattedValue = editValue.toLowerCase() === "true" || editValue === "1";
      } else if (type.includes("INT") || type.includes("NUM")) {
        const parsed = parseInt(editValue, 10);
        formattedValue = isNaN(parsed) ? editValue : parsed;
      } else if (type.includes("REAL") || type.includes("FLOAT") || type.includes("DOUBLE")) {
        const parsed = parseFloat(editValue);
        formattedValue = isNaN(parsed) ? editValue : parsed;
      }
    }

    if (formattedValue === originalValue) return;

    triggerStatus("saving", "Saving changes...");
    try {
      // Argument names match Rust `update_db_cell(uri, table, pk_column, pk_value, col_name, new_value)`
      await invoke("update_db_cell", {
        uri: filePath,
        table: activeTable,
        pkColumn: pkColumn.name,
        pkValue: pkValue,
        colName: colName,
        newValue: formattedValue,
      });
      triggerStatus("success", "Saved!");
      loadTableData(activeTable);
    } catch (err: any) {
      console.error("Failed to update cell:", err);
      triggerStatus("error", `Save failed: ${err.toString()}`);
    }
  };

  const handleAddRow = async () => {
    if (!activeTable) return;
    triggerStatus("saving", "Inserting new row...");
    try {
      await invoke("insert_db_row", {
        uri: filePath,
        table: activeTable,
      });
      triggerStatus("success", "New row added!");
      loadTableData(activeTable);
    } catch (err: any) {
      console.error("Failed to add row:", err);
      triggerStatus("error", `Failed to add row: ${err.toString()}`);
    }
  };

  const handleDeleteRow = async (origRowIndex: number) => {
    if (!activeTable || !data || !pkColumn || pkIndex === -1) return;

    const row = data.rows[origRowIndex];
    const pkValue = row[pkIndex];

    const performDelete = async () => {
      triggerStatus("saving", "Deleting row...");
      try {
        await invoke("delete_db_row", {
          uri: filePath,
          table: activeTable,
          pkColumn: pkColumn.name,
          pkValue: pkValue,
        });
        triggerStatus("success", "Row deleted!");
        if (triggerToast) triggerToast("Row deleted successfully!", "success");
        loadTableData(activeTable);
      } catch (err: any) {
        console.error("Failed to delete row:", err);
        triggerStatus("error", `Failed to delete row: ${err.toString()}`);
        if (triggerToast) triggerToast(`Failed to delete row: ${err}`, "error");
      }
    };

    if (triggerConfirm) {
      triggerConfirm(`Are you sure you want to delete this row (PK: ${pkValue})?`, performDelete);
    } else if (confirm(`Are you sure you want to delete this row (PK: ${pkValue})?`)) {
      await performDelete();
    }
  };

  const handleAddColumnSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!activeTable || !newColName.trim()) return;

    triggerStatus("saving", "Adding column...");
    try {
      await invoke("add_db_column", {
        uri: filePath,
        table: activeTable,
        colName: newColName.trim().replace(/[^a-zA-Z0-9_]/g, "_"),
        colType: newColType,
      });
      triggerStatus("success", `Column "${newColName}" added!`);
      setNewColName("");
      setShowAddColumn(false);
      loadTableData(activeTable);
    } catch (err: any) {
      console.error("Failed to add column:", err);
      triggerStatus("error", `Failed to add column: ${err.toString()}`);
    }
  };

  const handleRunQuery = async () => {
    if (!rawQuery.trim()) return;
    setIsQueryRunning(true);
    setQueryResult(null);
    triggerStatus("saving", "Running query...");
    try {
      const res = await invoke<any>("run_db_query", {
        options: { uri: filePath, auth_type: "basic" },
        query: rawQuery,
      });
      setQueryResult(res);
      const timing = res.execution_time_ms != null ? ` (${res.execution_time_ms} ms)` : "";
      triggerStatus("success", `Executed${timing}`);
      if (res.rows_affected !== null) {
        loadTables();
      }
    } catch (err: any) {
      console.error("Query failed:", err);
      setQueryResult({
        success: false,
        columns: null,
        rows: null,
        rows_affected: null,
        error: err.toString(),
      });
      triggerStatus("error", `Query failed: ${err.toString()}`);
    } finally {
      setIsQueryRunning(false);
    }
  };

  const handleExport = (format: "csv" | "json") => {
    const targetCols = viewMode === "query" ? queryResult?.columns : data?.columns;
    const targetRows =
      viewMode === "query" ? queryResult?.rows : visibleRows.map((v) => v.row);
    if (!targetCols || !targetRows) return;

    const baseName = `${activeTable || "query"}_export`;
    if (format === "json") {
      const objects = targetRows.map((row) => {
        const obj: Record<string, any> = {};
        targetCols.forEach((col, i) => {
          obj[col.name] = row[i];
        });
        return obj;
      });
      triggerFileDownload(`${baseName}.json`, JSON.stringify(objects, null, 2), "application/json");
    } else {
      const escapeCsv = (v: any) => {
        if (v === null || v === undefined) return "";
        const s = typeof v === "object" ? JSON.stringify(v) : String(v);
        return s.includes(",") || s.includes('"') || s.includes("\n")
          ? `"${s.replace(/"/g, '""')}"`
          : s;
      };
      const lines = [
        targetCols.map((c) => escapeCsv(c.name)).join(","),
        ...targetRows.map((r) => r.map(escapeCsv).join(",")),
      ];
      triggerFileDownload(`${baseName}.csv`, lines.join("\n"), "text/csv;charset=utf-8");
    }
    if (triggerToast) triggerToast(`Exported ${targetRows.length} rows (${format.toUpperCase()})`, "success");
  };

  const totalRows = data?.total_rows ?? null;
  const isNextDisabled =
    !data ||
    (totalRows !== null ? offset + limit >= totalRows : data.rows.length < limit);

  return (
    <div className="sqlite-editor-container">
      <div className="sqlite-editor-header">
        <div className="db-info">
          <Database size={16} className="db-icon text-accent" />
          <span className="db-name font-semibold">{fileName}</span>
          <span className="db-badge">{engine.label}</span>
          <span className="db-path text-muted">{filePath.replace(/:([^:@/]+)@/, ":••••••@")}</span>
        </div>

        <div className="view-mode-toggle">
          <button
            className={`btn-toggle ${viewMode === "table" ? "active" : ""}`}
            onClick={() => setViewMode("table")}
          >
            <Columns size={13} />
            <span>Data View</span>
          </button>
          <button
            className={`btn-toggle ${viewMode === "query" ? "active" : ""}`}
            onClick={() => setViewMode("query")}
          >
            <TerminalSquare size={13} />
            <span>Query Console</span>
          </button>
        </div>

        <div className="status-indicator-zone">
          {saveStatus === "saving" && (
            <span className="status-badge text-muted">
              <RefreshCw size={12} className="spin-animation" /> {statusMessage}
            </span>
          )}
          {saveStatus === "success" && (
            <span className="status-badge text-success">
              <Check size={12} /> {statusMessage}
            </span>
          )}
          {saveStatus === "error" && (
            <span className="status-badge text-danger" title={statusMessage}>
              <AlertCircle size={12} />{" "}
              {statusMessage.length > 36 ? statusMessage.slice(0, 36) + "..." : statusMessage}
            </span>
          )}
        </div>
      </div>

      <div className="sqlite-editor-body">
        <div className="sqlite-sidebar">
          <div className="sidebar-title">
            <FileSpreadsheet size={13} />
            <span>Collections / Tables ({tables.length})</span>
            <button className="refresh-btn" onClick={loadTables} title="Refresh database structure">
              <RefreshCw size={12} />
            </button>
          </div>
          <div className="tables-list">
            {tables.map((tbl) => (
              <button
                key={tbl}
                className={`table-list-item ${activeTable === tbl ? "active" : ""}`}
                onClick={() => handleSelectTable(tbl)}
              >
                <FileSpreadsheet size={13} className="tbl-icon" />
                <span className="tbl-name">{tbl}</span>
              </button>
            ))}
            {tables.length === 0 && !loading && (
              <div className="empty-tables text-muted">No tables/namespaces found.</div>
            )}
          </div>
        </div>

        <div className="sqlite-grid-area">
          {error && (
            <div className="sqlite-error-banner">
              <AlertCircle size={16} />
              <span>{error}</span>
            </div>
          )}

          {viewMode === "query" ? (
            <div
              className="query-view"
              style={{
                display: "flex",
                flexDirection: "column",
                height: "100%",
                padding: "16px",
                gap: "12px",
                overflow: "hidden",
              }}
            >
              <div
                className="query-editor-container"
                style={{ flex: "0 0 40%", display: "flex", flexDirection: "column", gap: "8px" }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span className="font-semibold text-accent">
                    {engine.label} Query Console
                  </span>
                  <div style={{ display: "flex", gap: "8px" }}>
                    {queryResult?.rows && (
                      <>
                        <button className="btn-toolbar" onClick={() => handleExport("csv")}>
                          <Download size={12} /> CSV
                        </button>
                        <button className="btn-toolbar" onClick={() => handleExport("json")}>
                          <Download size={12} /> JSON
                        </button>
                      </>
                    )}
                    <button
                      className="btn-primary"
                      onClick={handleRunQuery}
                      disabled={isQueryRunning || !rawQuery.trim()}
                      style={{
                        padding: "6px 16px",
                        display: "flex",
                        alignItems: "center",
                        gap: "6px",
                        borderRadius: "4px",
                        cursor: "pointer",
                        border: "none",
                      }}
                    >
                      {isQueryRunning ? <RefreshCw size={14} className="spin-animation" /> : <Play size={14} />}
                      Run
                    </button>
                  </div>
                </div>
                <div
                  style={{
                    flex: 1,
                    border: "1px solid var(--border-primary)",
                    borderRadius: "4px",
                    overflow: "hidden",
                    paddingTop: "4px",
                  }}
                >
                  <Editor
                    height="100%"
                    language={engine.queryLang}
                    theme="vs-dark"
                    value={rawQuery}
                    onChange={(value) => setRawQuery(value || "")}
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                      fontFamily: "Ubuntu Mono, monospace",
                      padding: { top: 8, bottom: 8 },
                      scrollBeyondLastLine: false,
                      wordWrap: "on",
                    }}
                  />
                </div>
              </div>

              <div
                className="query-results-container"
                style={{ flex: "1", display: "flex", flexDirection: "column", gap: "8px", overflow: "hidden" }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span className="font-semibold text-muted">Results</span>
                  {queryResult?.execution_time_ms != null && (
                    <span className="text-muted" style={{ fontSize: "11px" }}>
                      {queryResult.rows ? `${queryResult.rows.length} row(s) • ` : ""}
                      {queryResult.execution_time_ms} ms
                    </span>
                  )}
                </div>
                <div
                  style={{
                    flex: "1",
                    backgroundColor: "var(--bg-primary)",
                    border: "1px solid var(--border-color)",
                    borderRadius: "4px",
                    overflow: "auto",
                  }}
                >
                  {isQueryRunning ? (
                    <div style={{ display: "flex", height: "100%", alignItems: "center", justifyContent: "center", color: "var(--text-muted)" }}>
                      <RefreshCw size={24} className="spin-animation" />
                      <span style={{ marginLeft: "8px" }}>Executing...</span>
                    </div>
                  ) : !queryResult ? (
                    <div style={{ display: "flex", height: "100%", alignItems: "center", justifyContent: "center", color: "var(--text-muted)" }}>
                      Run a query or command above to inspect output.
                    </div>
                  ) : !queryResult.success ? (
                    <div className="query-log query-error">
                      <AlertCircle size={16} className="log-icon" style={{ marginRight: "8px", flexShrink: 0 }} />
                      <span>{queryResult.error}</span>
                    </div>
                  ) : queryResult.rows_affected !== null ? (
                    <div className="query-log query-success">
                      <Check size={16} className="log-icon" style={{ marginRight: "8px", flexShrink: 0 }} />
                      <span>
                        Executed successfully. {queryResult.rows_affected} row(s) / item(s) affected.
                      </span>
                    </div>
                  ) : queryResult.columns && queryResult.rows ? (
                    <div className="table-viewport" style={{ height: "100%" }}>
                      <table style={{ width: "100%" }}>
                        <thead>
                          <tr>
                            {queryResult.columns.map((col, idx) => (
                              <th key={idx}>
                                <div className="header-cell">
                                  <span className="col-name">{col.name}</span>
                                </div>
                              </th>
                            ))}
                          </tr>
                        </thead>
                        <tbody>
                          {queryResult.rows.map((row, rIdx) => (
                            <tr key={rIdx}>
                              {row.map((val, cIdx) => {
                                const text = formatCellDisplay(val);
                                return (
                                  <td
                                    key={cIdx}
                                    className={`grid-cell ${val === null ? "cell-null" : ""}`}
                                    onClick={() =>
                                      setInspectedCell({
                                        colName: queryResult.columns![cIdx].name,
                                        value: val,
                                      })
                                    }
                                    title="Click to inspect full value"
                                  >
                                    {val === null ? (
                                      <span className="null-tag">NULL</span>
                                    ) : (
                                      <span className="cell-value">{text}</span>
                                    )}
                                  </td>
                                );
                              })}
                            </tr>
                          ))}
                          {queryResult.rows.length === 0 && (
                            <tr>
                              <td colSpan={queryResult.columns.length} className="empty-rows text-muted">
                                No results returned.
                              </td>
                            </tr>
                          )}
                        </tbody>
                      </table>
                    </div>
                  ) : null}
                </div>
              </div>
            </div>
          ) : activeTable ? (
            <div className="table-grid-view">
              <div className="grid-toolbar">
                <div className="table-meta">
                  <span className="table-title font-semibold">{activeTable}</span>
                  <span className="row-count text-muted">
                    {visibleRows.length}
                    {totalRows !== null ? ` / ${totalRows}` : ""} rows
                  </span>
                </div>

                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "6px",
                    backgroundColor: "var(--bg-primary)",
                    border: "1px solid var(--border-primary)",
                    borderRadius: "4px",
                    padding: "3px 8px",
                    minWidth: "180px",
                  }}
                >
                  <Search size={12} className="text-muted" />
                  <input
                    type="text"
                    placeholder="Filter current page..."
                    value={searchFilter}
                    onChange={(e) => setSearchFilter(e.target.value)}
                    style={{
                      background: "transparent",
                      border: "none",
                      color: "var(--text-primary)",
                      fontSize: "12px",
                      outline: "none",
                      width: "100%",
                    }}
                  />
                </div>

                <div className="toolbar-actions">
                  {pkColumn ? (
                    engine.supportsSchemaAlter && (
                      <button
                        className="btn-toolbar btn-primary"
                        onClick={handleAddRow}
                        title="Add a new empty row"
                      >
                        <Plus size={13} />
                        <span>Add Row</span>
                      </button>
                    )
                  ) : (
                    <div
                      className="no-pk-warning"
                      title="Primary key is required to edit or delete rows."
                    >
                      <HelpCircle size={12} />
                      <span>Read-only (No PK)</span>
                    </div>
                  )}

                  {engine.supportsSchemaAlter && (
                    <button
                      className={`btn-toolbar ${showAddColumn ? "active" : ""}`}
                      onClick={() => setShowAddColumn(!showAddColumn)}
                      title="Add a new column"
                    >
                      <Columns size={13} />
                      <span>Add Column</span>
                    </button>
                  )}

                  <button
                    className="btn-toolbar"
                    onClick={() => handleExport("csv")}
                    title="Export visible rows as CSV"
                    disabled={!data || data.rows.length === 0}
                  >
                    <Download size={12} />
                    <span>CSV</span>
                  </button>

                  <button
                    className="btn-toolbar"
                    onClick={() => handleExport("json")}
                    title="Export visible rows as JSON"
                    disabled={!data || data.rows.length === 0}
                  >
                    <Download size={12} />
                    <span>JSON</span>
                  </button>

                  <button
                    className="btn-toolbar"
                    onClick={() => loadTableData(activeTable)}
                    title="Reload data"
                  >
                    <RefreshCw size={13} />
                  </button>
                </div>
              </div>

              {showAddColumn && (
                <form className="add-column-panel animate-fade-in" onSubmit={handleAddColumnSubmit}>
                  <div className="form-group">
                    <label>Column Name</label>
                    <input
                      type="text"
                      placeholder="e.g. created_at"
                      value={newColName}
                      onChange={(e) => setNewColName(e.target.value)}
                      required
                    />
                  </div>
                  <div className="form-group" style={{ minWidth: "130px" }}>
                    <label>Type</label>
                    <AlouetteSelect
                      value={newColType}
                      onChange={setNewColType}
                      searchable={false}
                      options={[
                        { value: "TEXT", label: "TEXT" },
                        { value: "INTEGER", label: "INTEGER" },
                        { value: "REAL", label: "REAL" },
                        { value: "BOOLEAN", label: "BOOLEAN" },
                        { value: "BLOB", label: "BLOB" },
                      ]}
                    />
                  </div>
                  <div className="form-buttons">
                    <button type="submit" className="btn-form btn-primary">
                      Add
                    </button>
                    <button
                      type="button"
                      className="btn-form btn-cancel"
                      onClick={() => setShowAddColumn(false)}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              )}

              {loading && !data ? (
                <div className="grid-loading">
                  <RefreshCw size={24} className="spin-animation" />
                  <span>Loading records...</span>
                </div>
              ) : data ? (
                <>
                  <div className="sqlite-table-container">
                    <table className="sqlite-table">
                      <thead>
                        <tr>
                          {data.columns.map((col, idx) => (
                            <th
                              key={col.name}
                              className={col.is_pk ? "pk-header" : ""}
                              onClick={() => {
                                if (sortColIdx === idx) setSortAsc(!sortAsc);
                                else {
                                  setSortColIdx(idx);
                                  setSortAsc(true);
                                }
                              }}
                              style={{ cursor: "pointer" }}
                              title="Click to sort current page"
                            >
                              <div className="header-cell">
                                {col.is_pk && <Key size={11} className="pk-icon text-accent" />}
                                <span className="col-name">{col.name}</span>
                                <span className="col-type text-muted">{col.data_type.toLowerCase()}</span>
                                <ArrowUpDown size={10} style={{ opacity: sortColIdx === idx ? 1 : 0.35 }} />
                              </div>
                            </th>
                          ))}
                          {pkColumn && <th className="actions-header">Actions</th>}
                        </tr>
                      </thead>
                      <tbody>
                        {visibleRows.map(({ row, origIdx }) => (
                          <tr key={origIdx}>
                            {row.map((val, cIdx) => {
                              const colName = data.columns[cIdx]?.name || `col_${cIdx}`;
                              const isEditing =
                                editingCell?.rowIndex === origIdx &&
                                editingCell?.colName === colName;
                              const displayText = formatCellDisplay(val);

                              return (
                                <td
                                  key={cIdx}
                                  className={`grid-cell ${val === null ? "cell-null" : ""} ${typeof val === "number" ? "text-right" : ""}`}
                                  onDoubleClick={() => {
                                    if (pkColumn) {
                                      setEditingCell({ rowIndex: origIdx, colName });
                                      setEditValue(val === null ? "" : displayText);
                                    }
                                  }}
                                  title={
                                    pkColumn
                                      ? "Double-click to edit cell"
                                      : "Read-only cell"
                                  }
                                >
                                  {isEditing ? (
                                    <input
                                      type="text"
                                      className="cell-input"
                                      value={editValue}
                                      onChange={(e) => setEditValue(e.target.value)}
                                      onBlur={() => handleCellSave(origIdx, colName, val)}
                                      onKeyDown={(e) => {
                                        if (e.key === "Enter") handleCellSave(origIdx, colName, val);
                                        if (e.key === "Escape") setEditingCell(null);
                                      }}
                                      autoFocus
                                    />
                                  ) : (
                                    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "6px" }}>
                                      {val === null ? (
                                        <span className="null-tag">NULL</span>
                                      ) : (
                                        <span className="cell-value">{displayText}</span>
                                      )}
                                      {displayText.length > 36 && (
                                        <button
                                          type="button"
                                          onClick={(e) => {
                                            e.stopPropagation();
                                            setInspectedCell({ colName, value: val });
                                          }}
                                          title="Inspect full value"
                                          style={{
                                            background: "transparent",
                                            border: "none",
                                            color: "var(--text-muted)",
                                            cursor: "pointer",
                                            padding: 0,
                                            flexShrink: 0,
                                          }}
                                        >
                                          <Maximize2 size={11} />
                                        </button>
                                      )}
                                    </div>
                                  )}
                                </td>
                              );
                            })}

                            {pkColumn && (
                              <td className="actions-cell">
                                <button
                                  className="delete-row-btn"
                                  onClick={() => handleDeleteRow(origIdx)}
                                  title="Delete this row"
                                >
                                  <Trash2 size={13} />
                                </button>
                              </td>
                            )}
                          </tr>
                        ))}
                        {visibleRows.length === 0 && (
                          <tr>
                            <td
                              colSpan={Math.max(1, data.columns.length + (pkColumn ? 1 : 0))}
                              className="empty-rows text-muted"
                            >
                              {data.rows.length === 0
                                ? "Table contains no records."
                                : "No rows match the current filter."}
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </div>

                  <div className="sqlite-pagination">
                    <div className="pagination-group">
                      <span>Rows per page:</span>
                      <div style={{ width: "86px" }}>
                        <AlouetteSelect
                          value={String(limit)}
                          onChange={(val) => {
                            setLimit(Number(val));
                            setOffset(0);
                          }}
                          searchable={false}
                          options={[
                            { value: "50", label: "50" },
                            { value: "100", label: "100" },
                            { value: "500", label: "500" },
                          ]}
                        />
                      </div>
                    </div>

                    <div className="pagination-group gap-4">
                      <span>
                        Showing {data.rows.length === 0 ? 0 : offset + 1} -{" "}
                        {offset + data.rows.length}
                        {totalRows !== null ? ` of ${totalRows}` : ""}
                      </span>
                      <div className="pagination-group">
                        <button
                          onClick={() => setOffset(Math.max(0, offset - limit))}
                          disabled={offset === 0}
                          className="pagination-btn"
                        >
                          <ChevronLeft size={16} />
                        </button>
                        <button
                          onClick={() => setOffset(offset + limit)}
                          disabled={isNextDisabled}
                          className="pagination-btn"
                        >
                          <ChevronRight size={16} />
                        </button>
                      </div>
                    </div>
                  </div>
                </>
              ) : null}
            </div>
          ) : (
            <div className="grid-empty-state">
              <Database size={32} className="empty-icon text-muted" />
              <h3>No Table Selected</h3>
              <p>Choose a table or collection from the sidebar to inspect records.</p>
            </div>
          )}
        </div>
      </div>

      {inspectedCell && (
        <div
          onClick={() => setInspectedCell(null)}
          style={{
            position: "fixed",
            inset: 0,
            backgroundColor: "rgba(0,0,0,0.65)",
            zIndex: 1000,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: "24px",
          }}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              backgroundColor: "var(--bg-secondary, #141418)",
              border: "1px solid var(--border-primary, #2a2a35)",
              borderRadius: "8px",
              width: "min(680px, 90vw)",
              maxHeight: "80vh",
              display: "flex",
              flexDirection: "column",
              overflow: "hidden",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                padding: "12px 16px",
                borderBottom: "1px solid var(--border-primary)",
              }}
            >
              <span style={{ fontWeight: 600, fontSize: "13px" }}>
                Cell Value — <code>{inspectedCell.colName}</code>
              </span>
              <button
                onClick={() => setInspectedCell(null)}
                style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}
              >
                <X size={16} />
              </button>
            </div>
            <pre
              style={{
                margin: 0,
                padding: "16px",
                overflow: "auto",
                fontFamily: "var(--font-mono)",
                fontSize: "12px",
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                color: "var(--text-primary)",
              }}
            >
              {typeof inspectedCell.value === "object" && inspectedCell.value !== null
                ? JSON.stringify(inspectedCell.value, null, 2)
                : (() => {
                    const s = formatCellDisplay(inspectedCell.value);
                    try {
                      if (s.startsWith("{") || s.startsWith("[")) {
                        return JSON.stringify(JSON.parse(s), null, 2);
                      }
                    } catch {}
                    return s;
                  })()}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
