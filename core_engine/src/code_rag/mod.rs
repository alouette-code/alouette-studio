pub mod db;
pub mod embedding;
pub mod extractor;
pub mod indexer;
pub mod language_resolver;
pub mod languages;
pub mod normalizer;
pub mod query;
pub mod seed;
pub mod tier;

// Parser module chỉ active khi feature tree-sitter được bật
#[cfg(feature = "tree-sitter")]
pub mod parser;

pub use db::{VectorDb, VectorEntry, VectorEntryMeta};
pub use embedding::EmbeddingModel;
pub use extractor::extract_functions;
pub use indexer::{IndexEvent, IndexPriority, Indexer, IndexerConfig};
pub use language_resolver::LanguageResolver;
pub use languages::{all_languages, extension_map, language_map};
pub use normalizer::normalize_entry;
pub use query::{QueryEngine, QueryResult};
pub use seed::seed_code_library;
pub use tier::{LanguageConfig, Tier};
