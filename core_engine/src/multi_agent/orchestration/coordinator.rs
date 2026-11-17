use crate::agent_harness::AgentHarness;
use crate::agent_harness::parser::ToolCall;
use super::state_machine::OrchestratorState;
use super::super::blackboard::memory_store::MemoryStore;
use super::super::types::config::{CoreConfig, WorkerConfig};
use super::super::worker::session::{WorkerSession, WorkerState};
use super::super::worker::prompt_builder::PromptBuilder;
use super::super::worker::tool_router::ToolRouter;
use super::super::errors::MultiAgentError;

/// Coordinator điều phối toàn bộ quá trình Multi-Agent
pub struct Coordinator<'a> {
    pub state: OrchestratorState,
    pub memory: MemoryStore,
    pub config: CoreConfig,
    harness: &'a mut AgentHarness,
}

impl<'a> Coordinator<'a> {
    pub fn new(harness: &'a mut AgentHarness) -> Self {
        Self {
            state: OrchestratorState::Init,
            memory: MemoryStore::new(),
            config: CoreConfig::default(),
            harness,
        }
    }

    pub fn with_config(harness: &'a mut AgentHarness, config: CoreConfig) -> Self {
        Self {
            state: OrchestratorState::Init,
            memory: MemoryStore::new(),
            config,
            harness,
        }
    }

    /// Khởi chạy chuỗi Workflow hoàn chỉnh với một Closure đóng vai trò LLM
    /// Khởi chạy chuỗi Workflow hoàn chỉnh với một Closure đóng vai trò LLM
    pub async fn run_workflow<F, Fut, P>(
        &mut self,
        task: &str,
        mut llm_executor: F,
        mut progress_handler: P,
    ) -> Result<String, MultiAgentError>
    where
        F: FnMut(String, Vec<crate::agent_harness::ChatMessage>) -> Fut,
        Fut: std::future::Future<Output = Result<(Option<String>, Vec<ToolCall>), String>>,
        P: FnMut(&str, &str, &str, u32, &str), // role_id, role_name, avatar, step, content
    {
        let res = self.run_workflow_internal(task, &mut llm_executor, &mut progress_handler).await;
        
        let plan_path = self.harness.get_workspace_root().join("implementation_plan.md");
        if res.is_err() || matches!(self.state, OrchestratorState::Finished(_)) {
            if plan_path.exists() {
                println!("[Coordinator] Cleaning up implementation_plan.md file...");
                let _ = std::fs::remove_file(&plan_path);
            }
        }
        res
    }

    /// Nội bộ thực thi Workflow
    async fn run_workflow_internal<F, Fut, P>(
        &mut self,
        task: &str,
        llm_executor: &mut F,
        progress_handler: &mut P,
    ) -> Result<String, MultiAgentError>
    where
        F: FnMut(String, Vec<crate::agent_harness::ChatMessage>) -> Fut,
        Fut: std::future::Future<Output = Result<(Option<String>, Vec<ToolCall>), String>>,
        P: FnMut(&str, &str, &str, u32, &str),
    {
        let plan_path = self.harness.get_workspace_root().join("implementation_plan.md");
        
        // Tự động phê duyệt toàn bộ thao tác (chạy lệnh/đọc/ghi) trong luồng Multi-Agent để chạy mượt mà không bị dừng ngắt quãng
        self.config.auto_approve_all = true;

        let plan_result = if plan_path.exists() {
            println!("[Coordinator] Technical plan found. Resuming workflow from Coding phase...");
            match std::fs::read_to_string(&plan_path) {
                Ok(content) => content,
                Err(e) => {
                    return Err(MultiAgentError::StateError(format!("Không thể đọc tệp kế hoạch implementation_plan.md: {}", e)));
                }
            }
        } else {
            // --- 1. PLANNING PHASE ---
            self.state = OrchestratorState::Planning;
            let planner_cfg = WorkerConfig {
                role_id: "secretary".to_string(),
                role_name: "Thư Ký".to_string(),
                avatar: "👩‍💼".to_string(),
                system_prompt: "Bạn là Thư Ký (Secretary 👩‍💼).\n\
                                Nhiệm vụ của bạn là nghiên cứu cấu trúc dự án và lập kế hoạch chi tiết để giải quyết yêu cầu của người dùng.\n\
                                Bạn được cấp phép sử dụng các công cụ nghiên cứu hệ thống, chạy lệnh terminal (như execute_command), và đọc tệp để tìm hiểu sâu về dự án.\n\
                                Bạn KHÔNG ĐƯỢC PHÉP chỉnh sửa bất kỳ tệp mã nguồn nào ngoài việc tạo bản kế hoạch kỹ thuật tại tệp 'implementation_plan.md' trong thư mục dự án (sử dụng công cụ 'write_file').\n\
                                BẮT BUỘC: Ngay sau khi đã ghi thành công bản kế hoạch vào tệp 'implementation_plan.md', bạn phải dừng lại, báo cáo tóm tắt kế hoạch kỹ thuật vừa tạo và KẾT THÚC lượt chạy của mình (không gọi thêm bất kỳ công cụ nào khác như spawn_subagent hay save_memory). Trả lời ngắn gọn và hoàn thành công việc.".to_string(),
                allowed_tools: vec![
                    "scan_directory_tree".to_string(),
                    "scan_subdirectory".to_string(),
                    "get_project_files".to_string(),
                    "read_file".to_string(),
                    "read_file_range".to_string(),
                    "read_chunk".to_string(),
                    "search_files".to_string(),
                    "search_symbol".to_string(),
                    "extract_symbol".to_string(),
                    "search_memory".to_string(),
                    "search_web".to_string(),
                    "fetch_webpage".to_string(),
                    "ping_zero_min".to_string(),
                    "write_file".to_string(),
                    "execute_command".to_string(),
                    "vm_execute_command".to_string(),
                    "check_port".to_string(),
                    "vm_list".to_string(),
                    "vm_start".to_string(),
                    "vm_stop".to_string(),
                    "kill_command".to_string(),
                    "check_command_status".to_string(),
                    "open_browser".to_string(),
                    "get_browser_elements".to_string(),
                    "browser_click".to_string(),
                    "browser_type".to_string(),
                    "browser_click_hardware".to_string(),
                    "browser_type_hardware".to_string(),
                    "read_screen_fallback".to_string(),
                ],
                max_iterations: 30,
                ..Default::default()
            };
            
            let res = self.run_worker(planner_cfg, task.to_string(), llm_executor, progress_handler).await?;
            self.memory.set("plan", res.clone()).await;
            progress_handler("secretary", "Thư Ký", "👩‍💼", 999, &res);
            
            // Tạm dừng ở đây, báo về cho người dùng duyệt kế hoạch
            self.state = OrchestratorState::Halted("Chờ phê duyệt kế hoạch".to_string());
            return Ok(format!(
                "Kế hoạch kỹ thuật đã được tạo. Vui lòng xem bản kế hoạch ở bảng 'AI Technical Plan' bên phải, chỉnh sửa nếu cần, rồi nhấn nút phê duyệt hoặc phản hồi để tôi bắt đầu thực thi code.\n\n{}",
                res
            ));
        };
        
        // --- 2. CODING PHASE ---
        self.state = OrchestratorState::Coding;
        let coder_task = format!("Yêu cầu ban đầu: {}\nKế Hoạch: {}\nThực thi kế hoạch này.", task, plan_result);
        
        let coder_cfg = WorkerConfig {
            role_id: "coder".to_string(),
            role_name: "Coder".to_string(),
            avatar: "💻".to_string(),
            system_prompt: "Bạn là Lập Trình Viên (Coder 💻).\n\
                            Nhiệm vụ của bạn là đọc bản kế hoạch kỹ thuật trong tệp 'implementation_plan.md' và thực hiện viết/sửa đổi mã nguồn tương ứng để hoàn thành yêu cầu.\n\
                            Hãy sử dụng các công cụ ghi/sửa tệp được cấp (như write_file, replace_in_file).\n\
                            Tuyệt đối KHÔNG sử dụng công cụ 'execute_command' vì vai trò của bạn không được cấp quyền chạy lệnh hệ thống.\n\
                            BẮT BUỘC: Khi đã hoàn thành viết và chỉnh sửa toàn bộ các mã nguồn theo kế hoạch, hãy báo cáo danh sách các tệp đã sửa đổi và KẾT THÚC lượt chạy của mình (không gọi thêm bất kỳ công cụ nào khác).".to_string(),
            allowed_tools: vec![
                "write_file".to_string(),
                "replace_in_file".to_string(),
                "scan_directory_tree".to_string(),
                "scan_subdirectory".to_string(),
                "get_project_files".to_string(),
                "read_file".to_string(),
                "read_file_range".to_string(),
                "read_chunk".to_string(),
                "search_files".to_string(),
                "search_symbol".to_string(),
                "extract_symbol".to_string(),
                "save_memory".to_string(),
                "search_memory".to_string(),
            ],
            max_iterations: 50,
            ..Default::default()
        };

        let code_result = self.run_worker(coder_cfg, coder_task, llm_executor, progress_handler).await?;
        self.memory.set("code_summary", code_result.clone()).await;

        // --- 3. VERIFYING PHASE ---
        self.state = OrchestratorState::Verifying;
        let verifier_task = format!(
            "Yêu cầu ban đầu: {}\nKế hoạch: {}\nKết quả Code: {}\nHãy chạy các lệnh kiểm thử hoặc test suite cần thiết để xác minh xem code đã hoạt động chính xác chưa.",
            task, plan_result, code_result
        );
        
        let verifier_cfg = WorkerConfig {
            role_id: "verifier".to_string(),
            role_name: "Verifier".to_string(),
            avatar: "🔍".to_string(),
            system_prompt: "Bạn là Người Kiểm Thử/Xác Minh (Verifier 🔍).\n\
                            Nhiệm vụ của bạn là chạy các lệnh kiểm thử hoặc test suite cần thiết (sử dụng công cụ 'execute_command') để xác minh xem các thay đổi mã nguồn của Coder có hoạt động chính xác và không gây lỗi cho hệ thống hay không.\n\
                            BẮT BUỘC: Sau khi đã chạy xong các lệnh kiểm thử và nhận được kết quả, hãy báo cáo kết quả kiểm tra rõ ràng và KẾT THÚC lượt chạy.".to_string(),
            allowed_tools: vec![
                "execute_command".to_string(),
                "scan_directory_tree".to_string(),
                "scan_subdirectory".to_string(),
                "get_project_files".to_string(),
                "read_file".to_string(),
                "read_file_range".to_string(),
                "read_chunk".to_string(),
                "search_memory".to_string(),
            ],
            max_iterations: 5,
            ..Default::default()
        };

        let verify_result = self.run_worker(verifier_cfg, verifier_task, llm_executor, progress_handler).await?;
        self.memory.set("verify_summary", verify_result.clone()).await;

        self.state = OrchestratorState::Finished("Workflow hoàn thành".to_string());
        Ok(format!("Plan: {}\n\nCode Result: {}\n\nVerification: {}", plan_result, code_result, verify_result))
    }

    /// Quản lý vòng đời của một Worker cụ thể
    async fn run_worker<F, Fut, P>(
        &mut self,
        config: WorkerConfig,
        task_prompt: String,
        llm_executor: &mut F,
        progress_handler: &mut P,
    ) -> Result<String, MultiAgentError>
    where
        F: FnMut(String, Vec<crate::agent_harness::ChatMessage>) -> Fut,
        Fut: std::future::Future<Output = Result<(Option<String>, Vec<ToolCall>), String>>,
        P: FnMut(&str, &str, &str, u32, &str),
    {
        // Khởi tạo Prompt và Session
        let system_prompt = PromptBuilder::build(
            self.harness,
            &config.system_prompt,
            &config.allowed_tools,
        );
        let mut session = WorkerSession::new(config.clone(), task_prompt);

        loop {
            // Kiểm tra tín hiệu hủy bỏ
            if self.harness.cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                self.state = OrchestratorState::Error("Workflow bị hủy bởi người dùng".to_string());
                return Err(MultiAgentError::Cancelled);
            }

            if session.has_reached_limit() {
                return Err(MultiAgentError::IterationLimitExceeded(config.role_name));
            }

            // Giao tiếp với LLM
            let (text_opt, tools) = llm_executor(system_prompt.clone(), session.history.clone())
                .await
                .map_err(|e| MultiAgentError::ToolError(format!("LLM Error: {}", e)))?;
            
            session.append_llm_response(text_opt.clone(), tools);

            if let Some(ref text) = text_opt {
                progress_handler(&config.role_id, &config.role_name, &config.avatar, session.iteration_count, text);
            }

            match session.state {
                WorkerState::Finished(_) => {
                    let final_res = text_opt.unwrap_or_else(|| "Hoàn thành không có output".to_string());
                    progress_handler(&config.role_id, &config.role_name, &config.avatar, 999, &final_res);
                    return Ok(final_res);
                }
                WorkerState::WaitingForToolResult(ref pending_tools) => {
                    let pending_tools = pending_tools.clone();
                    let mut router = ToolRouter::new(&mut self.harness);
                    let session_id = format!("worker_{}", config.role_id);
                    
                    for tool in &pending_tools {
                        // 1. Kiểm tra giới hạn quyền Tool (Role-based Tool Limits)
                        if !session.is_tool_allowed(&tool.name) {
                            let err_msg = format!("Lỗi bảo mật: Tool '{}' không được cấp phép cho vai trò '{}'.", tool.name, config.role_name);
                            session.append_tool_result(tool.call_id.clone().unwrap_or_default(), tool.name.clone(), err_msg.clone(), false);
                            progress_handler(&config.role_id, &config.role_name, &config.avatar, session.iteration_count, &err_msg);
                            continue;
                        }

                        // 1.5 Kiểm tra phê duyệt Tool (User Approval Check)
                        // Tự động phê duyệt viết tệp implementation_plan.md để chạy mượt mà lên kế hoạch
                        let needs_approval = if tool.name == "write_file" && tool.arguments.get("path").and_then(|v| v.as_str()).map_or(false, |p| p.contains("implementation_plan.md")) {
                            false
                        } else {
                            AgentHarness::tool_needs_approval(
                                tool,
                                self.config.auto_approve_all,
                                self.config.auto_approve_reads,
                                self.config.auto_approve_writes,
                            )
                        };

                        if needs_approval {
                            let halt_msg = format!(
                                "Tool '{}' yêu cầu phê duyệt từ người dùng nhưng đang chạy trong luồng Multi-Agent tự động.",
                                tool.name
                            );
                            println!("[Coordinator] Halted: {}", halt_msg);
                            self.state = OrchestratorState::Halted(halt_msg.clone());
                            progress_handler(&config.role_id, &config.role_name, &config.avatar, session.iteration_count, &format!("Tạm ngưng: {}", halt_msg));
                            return Err(MultiAgentError::StateError(halt_msg));
                        }

                        // 2. Chạy tool thật thông qua Tool Router
                        let tool_msg = format!("Đang gọi tool: {}", tool.name);
                        println!("[{}] {}", config.role_name, tool_msg);
                        progress_handler(&config.role_id, &config.role_name, &config.avatar, session.iteration_count, &tool_msg);

                        match router.execute(&session_id, tool).await {
                            Ok(res) => {
                                session.append_tool_result(tool.call_id.clone().unwrap_or_default(), tool.name.clone(), res, true);
                            }
                            Err(e) => {
                                session.append_tool_result(tool.call_id.clone().unwrap_or_default(), tool.name.clone(), format!("Tool Error: {}", e), false);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
