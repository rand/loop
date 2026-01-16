//! Hypergraph memory system with tiered evolution.
//!
//! The memory module provides persistent storage for knowledge in a hypergraph
//! structure with automatic tier evolution:
//!
//! - **Task tier**: Working memory for the current task
//! - **Session tier**: Accumulated knowledge during a session
//! - **LongTerm tier**: Persistent knowledge across sessions
//! - **Archive tier**: Decayed but preserved knowledge
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::memory::{SqliteMemoryStore, Node, NodeType, Tier};
//!
//! let store = SqliteMemoryStore::in_memory()?;
//!
//! // Add a fact
//! let fact = Node::new(NodeType::Fact, "The API uses JWT for auth")
//!     .with_confidence(0.95);
//! store.add_node(&fact)?;
//!
//! // Search for related knowledge
//! let results = store.search_content("authentication", 10)?;
//!
//! // Promote important facts
//! store.promote(&[fact.id], "Frequently accessed")?;
//! ```

mod schema;
mod store;
mod types;

pub use schema::{get_schema_version, initialize_schema, is_initialized, SCHEMA_VERSION};
pub use store::{EvolutionEntry, MemoryStats, SqliteMemoryStore};
pub use types::{
    ConsolidationResult, EdgeId, EdgeMember, EdgeType, HyperEdge, Node, NodeId, NodeQuery,
    NodeType, Provenance, ProvenanceSource, Tier,
};
