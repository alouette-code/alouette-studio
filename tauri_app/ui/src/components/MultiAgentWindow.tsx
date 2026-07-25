import React, { useState, useEffect } from 'react';
import { Plus, History, Clock, FolderOpen, FolderPlus, MessageSquare, Trash2, TableOfContents, X } from 'lucide-react';
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import ReactMarkdown from "react-markdown";
import { Project } from "../types";

interface AgentHistoryItem {
  session_id: string;
  title: string;
  created_at: string;
  model: string;
  mode: string;
  active_cwd: string;
}
import brandIcon from "./logo_alouette.png";
import { WindowControls } from "./WindowControls";
import WindowResizer from "./WindowResizer";
import AiAgent from "./AiAgent";

interface MultiAgentWindowProps {
  theme: "dark" | "light";
  setTheme: (t: "dark" | "light") => void;
}

export default function MultiAgentWindow({ theme }: MultiAgentWindowProps) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [activeProjectId, setActiveProjectId] = useState<string>("");
  
  // Derive active project and CWD
  const activeProject = projects.find(p => p.id === activeProjectId);
  const activeProjectCwd = activeProject?.cwd || activeProject?.name || "";  
  const [projectHistories, setProjectHistories] = useState<Record<string, AgentHistoryItem[]>>({});
  const [expandedProjects, setExpandedProjects] = useState<Record<string, boolean>>({});
  const [showAllHistory, setShowAllHistory] = useState<Record<string, boolean>>({});
  const [activeSessionId, setActiveSessionId] = useState<string>("");
  const [activeSessionData, setActiveSessionData] = useState<any>(null);
  const [sessionKey, setSessionKey] = useState<number>(0);

  const [generatedPlan, setGeneratedPlan] = useState<string>("");
  const [isPlanOpen, setIsPlanOpen] = useState<boolean>(false);

  useEffect(() => {
    let unlisten: any;
    const setupListener = async () => {
      unlisten = await listen<any>("multi-agent-step", (event) => {
        const payload = event.payload;
        // In the new execution engine, step 999 is emitted with the final output of an agent.
        if (payload && payload.agent_role === "planner" && payload.step === 999) {
          setGeneratedPlan(payload.content);
          setIsPlanOpen(true);
        }
      });
    };
    setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const loadProjects = async () => {
    try {
      const list = await invoke<Project[]>("get_projects");
      setProjects(list);
      if (list.length > 0 && !activeProjectId) {
        setActiveProjectId(list[0].id);
        setExpandedProjects(prev => ({ ...prev, [list[0].id]: true }));
      }

      // Fetch histories for all projects
      const histories: Record<string, AgentHistoryItem[]> = {};
      for (const proj of list) {
        try {
          const hist = await invoke<AgentHistoryItem[]>("agent_get_history", { projectId: proj.id });
          histories[proj.id] = hist;
        } catch (e) {
          console.error(`Failed to fetch history for project ${proj.id}`, e);
        }
      }
      setProjectHistories(histories);
    } catch (err) {
      console.error("Failed to load projects:", err);
    }
  };

  useEffect(() => {
    loadProjects();
  }, []);

  const handleImportProject = async () => {
    try {
      const selectedPath: string | null = await invoke("open_folder_dialog");
      if (selectedPath) {
        const normalizedFolder = selectedPath.replace(/\\/g, "/");
        const folderName = normalizedFolder.split("/").pop();
        const newId = "folder_" + Date.now();
        const newConfig = {
          id: newId,
          name: folderName || "Open Folder",
          cwd: normalizedFolder,
          command: "",
          args: [],
          auto_restart: false,
        };
        await invoke("register_project", { config: newConfig });
        await loadProjects();
        setActiveProjectId(newId);
      }
    } catch (err) {
      console.error("Failed to select folder:", err);
    }
  };

  const handleSelectProject = (proj: Project) => {
    setExpandedProjects(prev => ({ ...prev, [proj.id]: !prev[proj.id] }));
    
    if (activeProjectId !== proj.id) {
      setActiveProjectId(proj.id);
      setActiveSessionId(""); 
      setActiveSessionData(null);
      setGeneratedPlan("");
      setSessionKey(prev => prev + 1); // Force re-mount of AiAgent
    }
  };

  const handleNewChat = (proj: Project, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent triggering handleSelectProject
    setActiveProjectId(proj.id);
    setActiveSessionId(""); // Clear active session to start new
    setActiveSessionData(null);
    setGeneratedPlan("");
    setSessionKey(prev => prev + 1); // Force re-mount of AiAgent
  };

  const handleSelectHistory = async (proj: Project, historyItem: AgentHistoryItem) => {
    setActiveProjectId(proj.id);
    try {
      const data: any = await invoke("load_agent_session", { sessionId: historyItem.session_id });
      setActiveSessionId(historyItem.session_id);
      setActiveSessionData(data);
      
      // Extract plan from history if it exists
      let foundPlan = "";
      if (data && Array.isArray(data.history)) {
         // In simulated multi-agent, the plan is usually at iteration 2 with Planner_DraftPlan
         // or it's a message from 'planner' with 'Lên Kế Hoạch'. 
         // For a robust check, let's just find the first long message from planner.
         const planMsg = data.history.find((m: any) => m.role === "agent" && (m.content?.Text || "").includes("Kế Hoạch") && (m.content?.Text || "").length > 100);
         if (planMsg) {
             foundPlan = planMsg.content?.Text || "";
         } else {
             // Fallback: Check loop_iterations for tool Planner_DraftPlan
             const draftIter = data.loop_result?.iterations?.find((i: any) => i.tool_name === "Planner_DraftPlan");
             if (draftIter && draftIter.tool_result) foundPlan = draftIter.tool_result;
         }
      }
      setGeneratedPlan(foundPlan);
      
      setSessionKey(prev => prev + 1); // Force re-mount of AiAgent
    } catch (e) {
      console.error("Failed to load session:", e);
    }
  };

  const handleDeleteProject = async (proj: Project, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await invoke("deregister_project", { projectId: proj.id });
      // If deleted project is active, clear active
      if (activeProjectId === proj.id) {
        setActiveProjectId("");
        setActiveSessionId("");
        setActiveSessionData(null);
      }
      await loadProjects();
    } catch (err) {
      console.error("Failed to delete project:", err);
    }
  };

  return (
    <div className="multi-agent-window" data-theme={theme} style={{ display: 'flex', flexDirection: 'row', width: '100vw', height: '100vh', backgroundColor: 'var(--bg-color)', color: 'var(--text-color)', overflow: 'hidden' }}>
      
      {/* Left Sidebar */}
      <div className="multi-agent-sidebar" data-tauri-drag-region>
        {/* Sidebar Header (Draggable) */}
        <div className="multi-agent-sidebar-header" data-tauri-drag-region>
          <img src={brandIcon} className="multi-agent-logo" alt="Logo" data-tauri-drag-region />
          <span className="multi-agent-title" data-tauri-drag-region>multi agen</span>
        </div>

        {/* Sidebar Actions */}
        <div className="multi-agent-sidebar-actions">
          <button className="multi-agent-action-btn primary" onClick={handleImportProject}>
            <FolderPlus size={14} />
            <span>Import Project</span>
          </button>

          <button className="multi-agent-action-btn">
            <History size={14} />
            <span>Conversation History</span>
          </button>

          <button className="multi-agent-action-btn">
            <Clock size={14} />
            <span>Scheduled Tasks</span>
          </button>
        </div>

        {/* Sidebar Projects/Conversations List */}
        <div className="multi-agent-sidebar-list-container">
          <div className="multi-agent-sidebar-section-title">Projects</div>
          <div className="multi-agent-sidebar-folder">
            {projects.length === 0 && (
              <span style={{ fontSize: '11px', color: 'var(--text-muted)', padding: '4px 8px' }}>
                No projects imported.
              </span>
            )}
            {projects.map(proj => (
              <div key={proj.id} className="multi-agent-proj-group">
                <button 
                  className={`multi-agent-conv-item ${activeProjectId === proj.id && !activeSessionId ? 'active' : ''}`}
                  onClick={() => handleSelectProject(proj)}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flex: 1, minWidth: 0 }}>
                    <FolderOpen size={13} style={{ color: activeProjectId === proj.id ? 'var(--text-color, #ffffff)' : 'var(--text-muted)' }} />
                    <span className="conv-title">{proj.name}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                    <div 
                      className="multi-agent-inline-action"
                      onClick={(e) => handleDeleteProject(proj, e)}
                      title="Delete Project"
                    >
                      <Trash2 size={13} />
                    </div>
                    <div 
                      className="multi-agent-inline-action"
                      onClick={(e) => handleNewChat(proj, e)}
                      title="New Chat"
                    >
                      <Plus size={14} />
                    </div>
                  </div>
                </button>
                
                {/* Nested History */}
                {expandedProjects[proj.id] && projectHistories[proj.id] && projectHistories[proj.id].length > 0 && (
                  <div className="multi-agent-history-list">
                    {projectHistories[proj.id]
                      .slice(0, showAllHistory[proj.id] ? undefined : 5)
                      .map(hist => (
                        <button
                          key={hist.session_id}
                          className={`multi-agent-history-item ${activeSessionId === hist.session_id ? 'active' : ''}`}
                          onClick={() => handleSelectHistory(proj, hist)}
                        >
                          <MessageSquare size={12} />
                          <span className="history-title">{hist.title || "Untitled Chat"}</span>
                        </button>
                    ))}
                    {!showAllHistory[proj.id] && projectHistories[proj.id].length > 5 && (
                      <button 
                        className="multi-agent-show-more-btn"
                        onClick={() => setShowAllHistory(prev => ({ ...prev, [proj.id]: true }))}
                      >
                        Show {projectHistories[proj.id].length - 5} more...
                      </button>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Right Main Area */}
      <div className="multi-agent-main">
        <div className="multi-agent-main-header" data-tauri-drag-region>
          <div className="multi-agent-main-title" data-tauri-drag-region>
            {projects.find(p => p.id === activeProjectId)?.name || "No Project Selected"}
          </div>
          <WindowControls />
        </div>

        {/* AiAgent Component Area */}
        <div className="multi-agent-content" style={{ display: 'flex', flexDirection: 'row', backgroundColor: 'var(--bg-color)', width: '100%', height: '100%', overflow: 'hidden' }}>
          
          {/* Main Chat Area */}
          <div style={{ flex: 1, display: 'flex', justifyContent: 'center', position: 'relative', minWidth: 0, height: '100%' }}>
            {/* Floating Plan Toggle Button (only show if closed) */}
            {!isPlanOpen && (
              <button 
                onClick={() => setIsPlanOpen(true)}
                style={{ 
                  position: 'absolute', 
                  top: '16px', 
                  right: '24px', 
                  background: 'var(--bg-secondary)', 
                  border: '1px solid var(--border-primary)', 
                  color: generatedPlan ? 'var(--text-primary)' : 'var(--text-muted)', 
                  cursor: 'pointer', 
                  display: 'flex', 
                  alignItems: 'center', 
                  justifyContent: 'center',
                  padding: '8px', 
                  borderRadius: '8px',
                  opacity: generatedPlan ? 1 : 0.5,
                  zIndex: 90,
                  boxShadow: '0 2px 8px rgba(0,0,0,0.3)',
                  transition: 'all 0.2s'
                }}
                title="View AI Plan"
              >
                <TableOfContents size={20} />
              </button>
            )}

            <div style={{ width: '100%', maxWidth: '950px', height: '100%', display: 'flex', flexDirection: 'column' }}>
              <AiAgent 
                key={sessionKey}
                activeProjectId={activeProjectId}
                activeProjectCwd={activeProjectCwd}
                initialSessionData={activeSessionData}
                variant="full"
                isMultiAgentPage={true}
              />
            </div>
          </div>

          {/* AI Plan Side Panel */}
          {isPlanOpen && (
            <div style={{
              width: '450px',
              height: '100%',
              backgroundColor: 'var(--bg-secondary)',
              borderLeft: '1px solid var(--border-primary)',
              display: 'flex',
              flexDirection: 'column',
              flexShrink: 0
            }}>
              <div style={{ padding: '12px 16px', borderBottom: '1px solid var(--border-primary)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <TableOfContents size={16} />
                  <span style={{ fontWeight: 600, fontSize: '13px' }}>AI Technical Plan</span>
                </div>
                <button onClick={() => setIsPlanOpen(false)} style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}>
                  <X size={16} />
                </button>
              </div>
              <div style={{ padding: '16px', overflowY: 'auto', flex: 1, fontSize: '13px', lineHeight: 1.6 }} className="markdown-body">
                {generatedPlan ? (
                  <ReactMarkdown>{generatedPlan}</ReactMarkdown>
                ) : (
                  <div style={{ color: 'var(--text-muted)', fontStyle: 'italic', textAlign: 'center', marginTop: '20px' }}>
                    Chưa có kế hoạch nào được tạo.
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      <WindowResizer />
    </div>
  );
}
