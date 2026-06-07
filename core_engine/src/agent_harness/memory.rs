use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Types of memory entries matching Claude Code's memory system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryType {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "feedback")]
    Feedback,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "reference")]
    Reference,
}

/// A single memory file with frontmatter, matching the Claude Code format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub name: String,
    pub description: String,
    pub metadata: MemoryMetadata,
    pub content: String,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    #[serde(rename = "type")]
    pub mem_type: MemoryType,
}

/// Manages persistent file-based memory with consolidation and pruning
pub struct MemoryManager {
    workspace_root: PathBuf,
    memory_dir: PathBuf,
    team_memory_dir: PathBuf,
}

impl MemoryManager {
    pub fn new(workspace_root: &Path) -> Self {
        let memory_dir = workspace_root.join(".claude").join("memories");
        let team_memory_dir = memory_dir.join("team");

        let _ = fs::create_dir_all(&memory_dir);
        let _ = fs::create_dir_all(&team_memory_dir);

        Self {
            workspace_root: workspace_root.to_path_buf(),
            memory_dir,
            team_memory_dir,
        }
    }

    /// Load all personal memory files with frontmatter
    pub fn load_all_memories(&self) -> Vec<MemoryEntry> {
        let mut entries = self.load_from_dir(&self.memory_dir);
        // Also load team memories
        if self.team_memory_dir.exists() {
            let team_entries = self.load_from_dir(&self.team_memory_dir);
            entries.extend(team_entries);
        }
        entries
    }

    fn load_from_dir(&self, dir: &Path) -> Vec<MemoryEntry> {
        let mut entries = Vec::new();
        if !dir.is_dir() {
            return entries;
        }

        for entry in fs::read_dir(dir).ok().into_iter().flatten() {
            let entry = entry.ok().map(|e| e.path());
            if let Some(path) = entry {
                if path.extension().map_or(true, |e| e != "md") {
                    continue;
                }
                if let Some(mem) = self.parse_memory_file(&path) {
                    entries.push(mem);
                }
            }
        }

        // Sort by name for deterministic ordering
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    /// Parse a memory file with frontmatter (YAML-like)
    fn parse_memory_file(&self, path: &Path) -> Option<MemoryEntry> {
        let content = fs::read_to_string(path).ok()?;

        let mut lines = content.lines();
        let first_line = lines.next()?;
        if first_line.trim() != "---" {
            return None;
        }

        let mut frontmatter_lines = Vec::new();
        let mut body_lines = Vec::new();
        let mut in_frontmatter = true;

        for line in lines {
            if in_frontmatter {
                if line.trim() == "---" {
                    in_frontmatter = false;
                } else {
                    frontmatter_lines.push(line);
                }
            } else {
                body_lines.push(line);
            }
        }

        if in_frontmatter {
            return None;
        }

        let frontmatter_text = frontmatter_lines.join("\n");
        let body = body_lines.join("\n").trim().to_string();

        // Simple YAML-like frontmatter parser
        let mut name = String::new();
        let mut description = String::new();
        let mut mem_type = MemoryType::Reference;

        for line in frontmatter_text.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("name:") {
                name = val.trim().trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("description:") {
                description = val.trim().trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("type:") {
                mem_type = match val.trim() {
                    "user" => MemoryType::User,
                    "feedback" => MemoryType::Feedback,
                    "project" => MemoryType::Project,
                    _ => MemoryType::Reference,
                };
            }
        }

        Some(MemoryEntry {
            name,
            description,
            metadata: MemoryMetadata { mem_type },
            content: body,
            file_path: path.to_path_buf(),
        })
    }

    /// Save a new memory entry or update an existing one
    pub fn save_memory(
        &self,
        name: &str,
        description: &str,
        mem_type: MemoryType,
        content: &str,
    ) -> Result<PathBuf, String> {
        let slug_name = name
            .to_lowercase()
            .replace(' ', "-")
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
        let file_path = self.memory_dir.join(format!("{}.md", slug_name));

        let mem_content = format!(
            "---\nname: {}\ndescription: {}\nmetadata:\n  type: {}\n---\n\n{}",
            name,
            description,
            match mem_type {
                MemoryType::User => "user",
                MemoryType::Feedback => "feedback",
                MemoryType::Project => "project",
                MemoryType::Reference => "reference",
            },
            content
        );

        fs::write(&file_path, mem_content)
            .map(|_| file_path)
            .map_err(|e| format!("Failed to save memory: {}", e))
    }

    /// Delete a memory entry by name
    pub fn delete_memory(&self, name: &str) -> Result<(), String> {
        let slug_name = name.to_lowercase().replace(' ', "-");
        let file_path = self.memory_dir.join(format!("{}.md", slug_name));

        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| format!("Failed to delete memory: {}", e))
        } else {
            Err(format!("Memory '{}' not found", name))
        }
    }

    /// Perform dream memory consolidation: merge near-duplicates
    pub fn consolidate_memories(&self) -> Result<ConsolidationReport, String> {
        let memories = self.load_all_memories();
        let mut report = ConsolidationReport::default();

        // Group by similar descriptions and merge
        let mut seen: HashMap<String, Vec<MemoryEntry>> = HashMap::new();
        for mem in &memories {
            let key = mem
                .description
                .split('.')
                .next()
                .unwrap_or(&mem.description)
                .to_string();
            seen.entry(key).or_default().push(mem.clone());
        }

        for (_key, group) in seen.iter() {
            if group.len() > 1 {
                // Merge into first entry, delete rest
                let keep = &group[0];
                for duplicate in &group[1..] {
                    // Append content
                    let mut merged_content =
                        fs::read_to_string(&keep.file_path).unwrap_or_default();
                    let dup_content = fs::read_to_string(&duplicate.file_path).unwrap_or_default();
                    if !dup_content.is_empty() {
                        merged_content.push_str("\n\n---\n\n");
                        merged_content.push_str(&dup_content);
                    }
                    let _ = fs::write(&keep.file_path, merged_content);
                    let _ = fs::remove_file(&duplicate.file_path);
                    report.merged += 1;
                }
            }
        }

        report.total = memories.len();
        Ok(report)
    }

    /// Prune stale memories: remove memories that reference non-existent code
    pub fn prune_stale_memories(&self, codebase_files: &[String]) -> Result<PruneReport, String> {
        let memories = self.load_all_memories();
        let mut report = PruneReport::default();

        for mem in &memories {
            let mut updated_lines = Vec::new();
            let mut modified = false;

            for line in mem.content.lines() {
                let mut contains_stale_file = false;
                for code_file in codebase_files {
                    if line.contains(code_file) {
                        let absolute_code_file = self.workspace_root.join(code_file);
                        if !absolute_code_file.exists() {
                            contains_stale_file = true;
                            break;
                        }
                    }
                }

                if contains_stale_file {
                    modified = true;
                } else {
                    updated_lines.push(line);
                }
            }

            if modified {
                let new_content = updated_lines.join("\n");
                if new_content.trim().is_empty() {
                    let _ = fs::remove_file(&mem.file_path);
                    report.deleted += 1;
                } else {
                    let _ = self.save_memory(
                        &mem.name,
                        &mem.description,
                        mem.metadata.mem_type.clone(),
                        &new_content,
                    );
                }
            }
        }

        report.total = memories.len();
        Ok(report)
    }

    /// Search memories by keyword across name, description, and content
    /// Uses relevance scoring: name matches > description matches > content matches
    pub fn search_memories(&self, query: &str) -> Vec<MemoryEntry> {
        self.search_memories_ranked(query, 0.0)
            .into_iter()
            .map(|(entry, _score)| entry)
            .collect()
    }

    /// Search memories with relevance scoring (TF-IDF inspired).
    /// Returns entries sorted by relevance score descending.
    /// `min_score` filters out entries below threshold (0.0 = include all matches).
    pub fn search_memories_ranked(&self, query: &str, min_score: f64) -> Vec<(MemoryEntry, f64)> {
        let memories = self.load_all_memories();
        let query_lower = query.to_lowercase();

        // Split query into tokens for word-level matching
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(MemoryEntry, f64)> = memories
            .into_iter()
            .filter_map(|m| {
                let score = Self::calculate_relevance(&m, &query_lower, &query_tokens);
                if score > min_score {
                    Some((m, score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending, then by name for determinism
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.name.cmp(&b.0.name))
        });

        scored
    }

    /// Calculate relevance score for a memory entry against a query.
    /// Scoring:
    /// - Exact phrase match in name: +3.0 per occurrence
    /// - Token match in name: +2.0 per token
    /// - Exact phrase match in description: +2.0
    /// - Token match in description: +1.0 per token
    /// - Exact phrase match in content: +1.0
    /// - Token match in content: +0.3 per token (capped at 3.0)
    /// - Recent/project type bonus: +0.5
    fn calculate_relevance(entry: &MemoryEntry, query_lower: &str, query_tokens: &[&str]) -> f64 {
        let mut score: f64 = 0.0;

        let name_lower = entry.name.to_lowercase();
        let desc_lower = entry.description.to_lowercase();
        let content_lower = entry.content.to_lowercase();

        // ── Name field (heaviest weight) ──
        if name_lower.contains(query_lower) {
            score += 3.0;
        }
        for token in query_tokens {
            if !token.is_empty() && name_lower.contains(token) {
                score += 2.0;
            }
        }

        // ── Description field ──
        if desc_lower.contains(query_lower) {
            score += 2.0;
        }
        for token in query_tokens {
            if !token.is_empty() && desc_lower.contains(token) {
                score += 1.0;
            }
        }

        // ── Content field (lightest weight) ──
        if content_lower.contains(query_lower) {
            score += 1.0;
        }
        let mut token_matches: f64 = 0.0;
        for token in query_tokens {
            if !token.is_empty() && content_lower.contains(token) {
                token_matches += 0.3;
            }
        }
        score += token_matches.min(3.0); // Cap content token score

        // ── Type bonus: recent user/feedback memories are more relevant ──
        match entry.metadata.mem_type {
            MemoryType::User | MemoryType::Feedback => score += 0.5,
            MemoryType::Project => score += 0.3,
            MemoryType::Reference => {} // No bonus
        }

        score
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConsolidationReport {
    pub total: usize,
    pub merged: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PruneReport {
    pub total: usize,
    pub deleted: usize,
}
