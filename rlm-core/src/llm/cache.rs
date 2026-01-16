//! Prompt caching for LLM requests.
//!
//! Provides cache key generation and hit tracking for prompt caching
//! supported by providers like Anthropic.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::ChatMessage;

/// Cache key for a prompt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey(pub String);

impl CacheKey {
    /// Generate a cache key from messages and system prompt.
    pub fn generate(system: Option<&str>, messages: &[ChatMessage]) -> Self {
        let mut hasher = Sha256::new();

        // Include system prompt
        if let Some(s) = system {
            hasher.update(b"system:");
            hasher.update(s.as_bytes());
            hasher.update(b"\n");
        }

        // Include messages
        for msg in messages {
            hasher.update(format!("{}:", msg.role as u8).as_bytes());
            hasher.update(msg.content.as_bytes());
            hasher.update(b"\n");
        }

        let hash = hasher.finalize();
        CacheKey(format!("{:x}", hash))
    }

    /// Generate a cache key from raw content.
    pub fn from_content(content: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        CacheKey(format!("{:x}", hash))
    }
}

impl std::fmt::Display for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0[..16]) // Short form for display
    }
}

/// Cache entry metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cache key
    pub key: CacheKey,
    /// When the entry was created
    pub created_at: DateTime<Utc>,
    /// When the entry was last accessed
    pub last_accessed: DateTime<Utc>,
    /// Number of hits
    pub hit_count: u64,
    /// Token count (for cost estimation)
    pub token_count: u64,
    /// Model this was cached for
    pub model: String,
}

impl CacheEntry {
    pub fn new(key: CacheKey, model: impl Into<String>, token_count: u64) -> Self {
        let now = Utc::now();
        Self {
            key,
            created_at: now,
            last_accessed: now,
            hit_count: 0,
            token_count,
            model: model.into(),
        }
    }

    /// Record a cache hit.
    pub fn record_hit(&mut self) {
        self.hit_count += 1;
        self.last_accessed = Utc::now();
    }

    /// Check if entry is expired.
    pub fn is_expired(&self, ttl: Duration) -> bool {
        Utc::now() - self.created_at > ttl
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total tokens read from cache
    pub cached_tokens: u64,
    /// Estimated cost savings (USD)
    pub estimated_savings: f64,
    /// Number of active entries
    pub entry_count: u64,
}

impl CacheStats {
    /// Calculate hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Record a hit.
    pub fn record_hit(&mut self, tokens: u64, savings: f64) {
        self.hits += 1;
        self.cached_tokens += tokens;
        self.estimated_savings += savings;
    }

    /// Record a miss.
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }
}

/// Prompt cache tracker.
///
/// Tracks which prompts have been cached and their usage statistics.
/// Note: This tracks local awareness of provider-side caching, not
/// an actual cache implementation.
pub struct PromptCache {
    entries: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
    /// Time-to-live for cache entries (provider-dependent)
    ttl: Duration,
    /// Cost savings per cached token (default: 90% of input cost)
    savings_rate: f64,
}

impl PromptCache {
    /// Create a new prompt cache.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            // Anthropic cache TTL is 5 minutes
            ttl: Duration::minutes(5),
            // 90% savings on cached tokens
            savings_rate: 0.9,
        }
    }

    /// Create with custom TTL.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Create with custom savings rate.
    pub fn with_savings_rate(mut self, rate: f64) -> Self {
        self.savings_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Check if a key is likely cached.
    pub async fn is_cached(&self, key: &CacheKey) -> bool {
        let entries = self.entries.read().await;
        entries
            .get(key)
            .map(|e| !e.is_expired(self.ttl))
            .unwrap_or(false)
    }

    /// Record a cache creation (prompt was sent with cache control).
    pub async fn record_creation(
        &self,
        key: CacheKey,
        model: impl Into<String>,
        token_count: u64,
    ) {
        let mut entries = self.entries.write().await;
        entries.insert(key.clone(), CacheEntry::new(key, model, token_count));

        let mut stats = self.stats.write().await;
        stats.entry_count = entries.len() as u64;
    }

    /// Record a cache hit.
    pub async fn record_hit(
        &self,
        key: &CacheKey,
        tokens_saved: u64,
        cost_per_token: f64,
    ) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(key) {
            entry.record_hit();
        }

        let savings = tokens_saved as f64 * cost_per_token * self.savings_rate;
        let mut stats = self.stats.write().await;
        stats.record_hit(tokens_saved, savings);
    }

    /// Record a cache miss.
    pub async fn record_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.record_miss();
    }

    /// Get cache statistics.
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Clean up expired entries.
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        entries.retain(|_, e| !e.is_expired(self.ttl));

        let mut stats = self.stats.write().await;
        stats.entry_count = entries.len() as u64;
    }

    /// Get all active entries.
    pub async fn entries(&self) -> Vec<CacheEntry> {
        let entries = self.entries.read().await;
        entries
            .values()
            .filter(|e| !e.is_expired(self.ttl))
            .cloned()
            .collect()
    }

    /// Clear all cache tracking.
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();

        let mut stats = self.stats.write().await;
        *stats = CacheStats::default();
    }
}

impl Default for PromptCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine optimal cache breakpoints in a message sequence.
///
/// Anthropic caching requires minimum 1024 tokens for cache-eligible content.
/// This function identifies where to place cache control markers.
pub fn find_cache_breakpoints(
    system: Option<&str>,
    messages: &[ChatMessage],
    min_tokens: usize,
) -> Vec<usize> {
    let mut breakpoints = Vec::new();
    let mut cumulative_chars = 0;

    // Approximate tokens as chars / 4
    let chars_threshold = min_tokens * 4;

    // Check system prompt
    if let Some(s) = system {
        cumulative_chars += s.len();
        if cumulative_chars >= chars_threshold {
            // System prompt itself is cache-eligible
            breakpoints.push(0); // Special marker for system
        }
    }

    // Check messages
    for (i, msg) in messages.iter().enumerate() {
        cumulative_chars += msg.content.len();
        if cumulative_chars >= chars_threshold && !breakpoints.contains(&(i + 1)) {
            breakpoints.push(i + 1);
        }
    }

    breakpoints
}

/// Apply cache control markers to messages.
pub fn apply_cache_markers(messages: &mut [ChatMessage], breakpoints: &[usize]) {
    for &bp in breakpoints {
        if bp > 0 && bp <= messages.len() {
            messages[bp - 1].cache_control = Some(super::types::CacheControl::Ephemeral);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::ChatRole;

    #[test]
    fn test_cache_key_generation() {
        let messages = vec![
            ChatMessage {
                role: ChatRole::User,
                content: "Hello".to_string(),
                cache_control: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "Hi there".to_string(),
                cache_control: None,
            },
        ];

        let key1 = CacheKey::generate(Some("System prompt"), &messages);
        let key2 = CacheKey::generate(Some("System prompt"), &messages);
        let key3 = CacheKey::generate(Some("Different prompt"), &messages);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_key_from_content() {
        let key1 = CacheKey::from_content("Hello world");
        let key2 = CacheKey::from_content("Hello world");
        let key3 = CacheKey::from_content("Goodbye world");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_entry_expiry() {
        let key = CacheKey::from_content("test");
        let entry = CacheEntry::new(key, "claude", 1000);

        // Fresh entry should not be expired
        assert!(!entry.is_expired(Duration::minutes(5)));

        // Entry should be expired with zero TTL
        assert!(entry.is_expired(Duration::zero()));
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);

        stats.record_hit(1000, 0.01);
        stats.record_hit(1000, 0.01);
        stats.record_miss();

        // 2 hits, 1 miss = 66.7% hit rate
        assert!((stats.hit_rate() - 0.667).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_prompt_cache_operations() {
        let cache = PromptCache::new();

        let key = CacheKey::from_content("test prompt");
        assert!(!cache.is_cached(&key).await);

        cache.record_creation(key.clone(), "claude", 1000).await;
        assert!(cache.is_cached(&key).await);

        cache.record_hit(&key, 1000, 0.000003).await;

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.cached_tokens, 1000);
        assert!(stats.estimated_savings > 0.0);
    }

    #[test]
    fn test_find_cache_breakpoints() {
        let messages = vec![
            ChatMessage::user("Short message"),
            ChatMessage::assistant("Another short one"),
            ChatMessage::user("A".repeat(5000)), // Long message
        ];

        // With 1024 token minimum (~4096 chars)
        let breakpoints = find_cache_breakpoints(None, &messages, 1024);

        // Should find a breakpoint after the long message
        assert!(!breakpoints.is_empty());
    }

    #[test]
    fn test_apply_cache_markers() {
        let mut messages = vec![
            ChatMessage::user("Message 1"),
            ChatMessage::assistant("Message 2"),
            ChatMessage::user("Message 3"),
        ];

        let breakpoints = vec![2]; // After message 2
        apply_cache_markers(&mut messages, &breakpoints);

        assert!(messages[0].cache_control.is_none());
        assert!(messages[1].cache_control.is_some());
        assert!(messages[2].cache_control.is_none());
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache = PromptCache::new().with_ttl(Duration::zero());

        let key = CacheKey::from_content("test");
        cache.record_creation(key.clone(), "claude", 1000).await;

        // Entry should be expired immediately with zero TTL
        cache.cleanup().await;

        let stats = cache.stats().await;
        assert_eq!(stats.entry_count, 0);
    }
}
