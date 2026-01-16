//! Topos integration module for rlm-core.
//!
//! This module provides integration with the Topos semantic contract language,
//! enabling bidirectional linking between Topos specifications and Lean formalizations.
//!
//! ## Overview
//!
//! The Topos integration supports the dual-track specification workflow:
//! - **Topos** (.tps files): Human-readable semantic contracts
//! - **Lean** (.lean files): Machine-verifiable formal specifications
//!
//! ## Linking Mechanism
//!
//! Topos specs reference Lean artifacts via `@lean` annotations:
//!
//! ```text
//! Concept Order:
//!   id: `OrderId`
//!   items: list of `OrderItem`
//!   @lean: specs/Order.lean#Order
//! ```
//!
//! Lean files include reverse links via comments:
//!
//! ```text
//! /--
//! @topos: OrderManagement.tps#Order
//! Order represents a customer order.
//! -/
//! structure Order where
//!   id : Nat
//!   items : List OrderItem
//! ```
//!
//! ## Components
//!
//! - [`types`]: Core types for references and links (`ToposRef`, `LeanRef`, `Link`)
//! - [`parser`]: Annotation parser for `@lean` and `@topos` annotations
//! - [`index`]: Bidirectional link index with persistence
//! - [`client`]: MCP client for the Topos server
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::topos::{LinkIndex, IndexBuilder, ToposClient};
//!
//! // Build index from project files
//! let index = IndexBuilder::new("/path/to/project")
//!     .build()
//!     .expect("Failed to build index");
//!
//! // Query links
//! let topos_ref = ToposRef::new("spec.tps", "Order");
//! for link in index.get_lean_refs(&topos_ref) {
//!     println!("Order links to: {}", link.lean);
//! }
//!
//! // Use MCP client for validation
//! let client = ToposClient::from_env();
//! client.connect().await?;
//! let result = client.validate_spec(Path::new("spec.tps")).await?;
//! ```

pub mod client;
pub mod index;
pub mod parser;
pub mod types;

// Re-exports for convenience
pub use client::{
    CompiledContext, Diagnostic, DiagnosticSeverity, SpecSummary, ToposClient, ToposClientConfig,
    ValidationResult,
};
pub use index::{IndexBuilder, IndexMetadata, LinkIndex};
pub use parser::{AnnotationParser, AnnotationType, ParsedAnnotation};
pub use types::{LeanRef, Link, LinkMetadata, LinkSource, LinkType, ToposElementType, ToposRef};
