use crate::agent_harness::AgentHarness;

/// Xây dựng System Prompt động dựa trên Role và Allowed Tools
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(harness: &AgentHarness, role_system_prompt: &str, allowed_tools: &[String]) -> String {
        let mut prompt = String::new();
        
        // 1. Identity & Role
        prompt.push_str(&format!("{}\n\n", role_system_prompt));

        // 2. Tích hợp Memory Prompt (AI Homes Feature)
        prompt.push_str("## Memory & Kỹ Năng\n");
        prompt.push_str("Hệ thống hỗ trợ Bộ nhớ dài hạn và RAG. Nếu bạn được cấp các công cụ như `save_memory` hoặc `search_memory`, hãy sử dụng chúng để lưu trữ và truy xuất các thói quen, phong cách code hoặc kiến thức dự án.\n");
        prompt.push_str("Tuân thủ chặt chẽ việc kiểm tra kỹ thông tin trước khi áp dụng.\n\n");

        // 3. Strict Constraints
        prompt.push_str("## Hướng dẫn cốt lõi\n");
        prompt.push_str("- Luôn bọc suy nghĩ vào thẻ <thought> trước khi gọi Tool.\n");
        prompt.push_str("- Cung cấp phản hồi ngắn gọn, đi thẳng vào vấn đề.\n");
        prompt.push_str("- Khi một Tool thất bại, hãy đọc kỹ gợi ý tự sửa lỗi (Self-Heal Suggestion) đi kèm thông báo lỗi để khắc phục.\n");
        prompt.push_str("- TUYỆT ĐỐI KHÔNG SỬ DỤNG CÁC CÔNG CỤ NGOÀI DANH SÁCH DƯỚI ĐÂY.\n\n");

        // 4. Tool Definitions
        prompt.push_str("## Công cụ được cấp phép (Allowed Tools)\n");
        if allowed_tools.is_empty() {
            prompt.push_str("Bạn có toàn quyền sử dụng tất cả công cụ (Full Access).\n");
        } else {
            prompt.push_str("Bạn CHỈ ĐƯỢC PHÉP sử dụng các công cụ sau:\n");
            for tool in allowed_tools {
                prompt.push_str(&format!("- {}\n", tool));
            }
        }
        
        // 5. Thêm Project Context chung từ Harness
        let claude_md_path = harness.get_workspace_root().join("CLAUDE.md");
        if claude_md_path.exists() {
            if let Ok(claude_context) = std::fs::read_to_string(&claude_md_path) {
                prompt.push_str("\n\n## Project-Specific Guidelines (CLAUDE.md)\n");
                prompt.push_str(&claude_context);
            }
        }

        // 6. Current environment
        let git_branch = harness.get_git_branch();
        let git_status = harness.get_git_status();

        prompt.push_str(&format!(
            "\n\n## Current Environment\n- OS: {}\n- Workspace: {}\n",
            std::env::consts::OS,
            harness.get_workspace_root().display()
        ));

        if !git_branch.is_empty() {
            prompt.push_str(&format!("- Branch: {}\n", git_branch));
        }
        if !git_status.is_empty() {
            prompt.push_str(&format!("\n## Git Status\n```\n{}\n```\n", git_status));
        }

        prompt
    }
}

