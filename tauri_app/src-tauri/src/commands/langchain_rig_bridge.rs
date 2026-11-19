use async_trait::async_trait;
use langchain_rust::{
    language_models::llm::LLM,
    schemas::Message as LcMessage,
};
use langchain_rust::language_models::{GenerateResult, LLMError};
use langchain_rust::schemas::StreamData;
use langchain_rust::tools::Tool as LcTool;
use rig_core::agent::AgentBuilder;
use rig_core::providers;
use rig_core::client::CompletionClient;
use rig_core::completion::Prompt;
use std::sync::Arc;
use serde_json::Value;

use futures_util::Stream;
use std::pin::Pin;

// ==========================================
// 1. LỚP WRAPPER CHO LLM CỦA RIG
// ==========================================
#[derive(Clone)]
#[allow(dead_code)]
pub struct RigLLMWrapper {
    pub provider: String,
    pub api_key: String,
    pub model_name: String,
}

#[async_trait]
impl LLM for RigLLMWrapper {
    async fn invoke(&self, prompt: &str) -> Result<String, LLMError> {
        let map_err = |e: String| LLMError::OtherError(e);
        let result = match self.provider.as_str() {
            "openai" => {
                let client = providers::openai::Client::new(&self.api_key)
                    .map_err(|e| map_err(format!("Failed to create OpenAI client: {}", e)))?;
                let model = client.completion_model(&self.model_name);
                let agent = AgentBuilder::new(model).build();
                agent
                    .prompt(prompt)
                    .await
                    .map_err(|e| map_err(format!("OpenAI prompt failed: {}", e)))?
            }
            "gemini" => {
                let client = providers::gemini::Client::new(&self.api_key)
                    .map_err(|e| map_err(format!("Failed to create Gemini client: {}", e)))?;
                let model = client.completion_model(&self.model_name);
                let agent = AgentBuilder::new(model).build();
                agent
                    .prompt(prompt)
                    .await
                    .map_err(|e| map_err(format!("Gemini prompt failed: {}", e)))?
            }
            "claude" => {
                let client = providers::anthropic::Client::new(&self.api_key)
                    .map_err(|e| map_err(format!("Failed to create Claude client: {}", e)))?;
                let model = client.completion_model(&self.model_name);
                let agent = AgentBuilder::new(model).build();
                agent
                    .prompt(prompt)
                    .await
                    .map_err(|e| map_err(format!("Claude prompt failed: {}", e)))?
            }
            other => {
                return Err(LLMError::OtherError(format!(
                    "Unsupported LLM provider: {}",
                    other
                )));
            }
        };
        Ok(result)
    }

    async fn generate(&self, messages: &[LcMessage]) -> Result<GenerateResult, LLMError> {
        let mut conversation = String::new();
        for msg in messages {
            let role_name = match msg.message_type {
                langchain_rust::schemas::MessageType::SystemMessage => "System",
                langchain_rust::schemas::MessageType::HumanMessage => "User",
                langchain_rust::schemas::MessageType::AIMessage => "AI",
                langchain_rust::schemas::MessageType::ToolMessage => "Tool",
            };
            conversation.push_str(&format!("{}: {}\n", role_name, msg.content));
        }
        let result = self.invoke(&conversation).await?;

        let mut res = GenerateResult::default();
        res.generation = result;
        Ok(res)
    }

    async fn stream(
        &self,
        _messages: &[LcMessage],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        Err(LLMError::OtherError(
            "Streaming is not supported for RigLLMWrapper".to_string(),
        ))
    }
}

// ==========================================
// 2. LỚP WRAPPER CHO TOOL
// ==========================================
#[allow(dead_code)]
pub struct RigToolWrapper {
    pub name: String,
    pub description: String,
    pub exec_func: Arc<dyn Fn(Value) -> Result<String, String> + Send + Sync>,
}

#[async_trait]
impl LcTool for RigToolWrapper {
    fn name(&self) -> String {
        self.name.clone()
    }
    
    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn std::error::Error>> {
        match (self.exec_func)(input) {
            Ok(res) => Ok(res),
            Err(e) => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))),
        }
    }
}
