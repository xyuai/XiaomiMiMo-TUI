//! Configuration loading and defaults for XiaomiMiMo TUI.

use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::audit::log_sensitive_event;
use crate::features::{Features, FeaturesToml, is_known_feature_key};
use crate::hooks::HooksConfig;

pub const DEFAULT_MAX_SUBAGENTS: usize = 5;
pub const MAX_SUBAGENTS: usize = 20;
pub const DEFAULT_TEXT_MODEL: &str = "mimo-v2.5-pro";
pub const DEFAULT_NVIDIA_NIM_MODEL: &str = "mimo-v2.5-pro";
pub const DEFAULT_NVIDIA_NIM_FLASH_MODEL: &str = "mimo-v2-flash";
pub const DEFAULT_NVIDIA_NIM_BASE_URL: &str = "https://integrate.api.nvidia.com/v1";
pub const DEFAULT_OPENROUTER_MODEL: &str = "mimo-v2.5-pro";
pub const DEFAULT_OPENROUTER_FLASH_MODEL: &str = "mimo-v2-flash";
pub const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const DEFAULT_NOVITA_MODEL: &str = "mimo-v2.5-pro";
pub const DEFAULT_NOVITA_FLASH_MODEL: &str = "mimo-v2-flash";
pub const DEFAULT_NOVITA_BASE_URL: &str = "https://api.novita.ai/v1";
pub const DEFAULT_FIREWORKS_MODEL: &str = "mimo-v2.5-pro";
pub const DEFAULT_FIREWORKS_BASE_URL: &str = "https://api.fireworks.ai/inference/v1";
pub const DEFAULT_SGLANG_MODEL: &str = "mimo-v2.5-pro";
pub const DEFAULT_SGLANG_FLASH_MODEL: &str = "mimo-v2-flash";
pub const DEFAULT_SGLANG_BASE_URL: &str = "http://localhost:30000/v1";
const API_KEYRING_SENTINEL: &str = "__KEYRING__";
pub const COMMON_XIAOMIMIMO_MODELS: &[&str] = &[
    "mimo-v2.5-pro",
    "mimo-v2.5",
    "mimo-v2-pro",
    "mimo-v2-omni",
    "mimo-v2-flash",
    "mimo-v2-tts",
    "mimo-v2.5-tts",
    "mimo-v2.5-tts-voicedesign",
    "mimo-v2.5-tts-voiceclone",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiProvider {
    XiaomiMiMo,
    NvidiaNim,
    Openrouter,
    Novita,
    Fireworks,
    Sglang,
}

impl ApiProvider {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "xiaomimimo" | "xiaomi-mimo" | "xiaomi_mimo" | "mimo" => Some(Self::XiaomiMiMo),
            "nvidia" | "nvidia-nim" | "nvidia_nim" | "nim" => Some(Self::NvidiaNim),
            "openrouter" | "open_router" => Some(Self::Openrouter),
            "novita" => Some(Self::Novita),
            "fireworks" | "fireworks-ai" => Some(Self::Fireworks),
            "sglang" | "sg-lang" => Some(Self::Sglang),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::XiaomiMiMo => "xiaomimimo",
            Self::NvidiaNim => "nvidia-nim",
            Self::Openrouter => "openrouter",
            Self::Novita => "novita",
            Self::Fireworks => "fireworks",
            Self::Sglang => "sglang",
        }
    }

    /// Human-friendly label for picker UIs / status chips.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::XiaomiMiMo => "XiaomiMiMo",
            Self::NvidiaNim => "NVIDIA NIM",
            Self::Openrouter => "OpenRouter",
            Self::Novita => "Novita AI",
            Self::Fireworks => "Fireworks AI",
            Self::Sglang => "SGLang",
        }
    }

    /// All providers, in the order shown in the picker.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::XiaomiMiMo,
            Self::NvidiaNim,
            Self::Openrouter,
            Self::Novita,
            Self::Fireworks,
            Self::Sglang,
        ]
    }
}

// ============================================================================
// Provider Capability Matrix
// ============================================================================

/// Known capabilities for a provider + resolved-model combination.
///
/// Returned by [`provider_capability`] to describe what a given provider
/// supports for the resolved model string.  All fields are derived from
/// static knowledge (release docs, API guides) rather than live API probes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ProviderCapability {
    /// Canonical provider identifier.
    pub provider: ApiProvider,
    /// Resolved model identifier that will be sent in the API payload.
    pub resolved_model: String,
    /// Context window in tokens (the maximum input the model can accept).
    pub context_window: u32,
    /// Recommended maximum output tokens (`max_tokens`) for this combo.
    pub max_output: u32,
    /// Whether the provider+model supports thinking/reasoning mode.
    pub thinking_supported: bool,
    /// Whether the provider returns prompt-cache telemetry fields.
    pub cache_telemetry_supported: bool,
    /// Which request-payload dialect the provider uses.
    pub request_payload_mode: RequestPayloadMode,
    /// Deprecation notice for legacy model aliases (empty when not deprecated).
    pub deprecation: Option<ModelDeprecation>,
}

/// Which request-payload dialect the provider speaks.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum RequestPayloadMode {
    /// Standard OpenAI-compatible `/v1/chat/completions` payload.
    ChatCompletions,
    /// Anthropic-style Responses API (XiaomiMiMo experimental).
    ResponsesApi,
}

/// Deprecation metadata for a legacy model alias.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ModelDeprecation {
    /// Legacy alias that is deprecated (e.g. "xiaomimimo-chat").
    pub alias: String,
    /// Canonical replacement model.
    pub replacement: String,
    /// Human-readable deprecation date / notice.
    pub notice: String,
}

/// Known deprecations for legacy XiaomiMiMo model aliases.
fn xiaomimimo_legacy_aliases() -> &'static [ModelDeprecation] {
    use std::sync::OnceLock;
    static ALIASES: OnceLock<Vec<ModelDeprecation>> = OnceLock::new();
    ALIASES.get_or_init(|| {
        vec![
            ModelDeprecation {
                alias: String::from("xiaomimimo-chat"),
                replacement: String::from("mimo-v2-flash"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2-flash' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-reasoner"),
                replacement: String::from("mimo-v2-flash"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2-flash' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-r1"),
                replacement: String::from("mimo-v2-flash"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2-flash' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-v3"),
                replacement: String::from("mimo-v2-flash"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2-flash' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-v3.2"),
                replacement: String::from("mimo-v2-flash"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2-flash' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-v4"),
                replacement: String::from("mimo-v2.5-pro"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2.5-pro' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-v4-pro"),
                replacement: String::from("mimo-v2.5-pro"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2.5-pro' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-v4pro"),
                replacement: String::from("mimo-v2.5-pro"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2.5-pro' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-v4-flash"),
                replacement: String::from("mimo-v2-flash"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2-flash' instead."),
            },
            ModelDeprecation {
                alias: String::from("xiaomimimo-v4flash"),
                replacement: String::from("mimo-v2-flash"),
                notice: String::from("Deprecated legacy alias; use official Xiaomi MiMo model ID 'mimo-v2-flash' instead."),
            },
        ]
    })
}

/// Check if a model name is a known legacy alias and return its deprecation info.
///
/// This matches the same list as [`canonical_model_name`] and
/// [`normalize_model_name`], returning deprecation metadata for each alias.
#[must_use]
pub fn deprecation_for_model(model: &str) -> Option<&'static ModelDeprecation> {
    let lower = model.trim().to_ascii_lowercase();
    xiaomimimo_legacy_aliases()
        .iter()
        .find(|d| d.alias == lower)
}

/// Resolve the provider capability for a given [`ApiProvider`] and resolved
/// model string.
///
/// The `resolved_model` should be the final model identifier that will appear
/// in the API payload (after normalization / provider-specific mapping).
#[must_use]
pub fn provider_capability(provider: ApiProvider, resolved_model: &str) -> ProviderCapability {
    let model_lower = canonical_model_name(resolved_model)
        .map(str::to_string)
        .unwrap_or_else(|| resolved_model.trim().to_ascii_lowercase());

    let is_tts = model_lower.contains("tts");
    let is_1m_text = model_lower.contains("mimo-v2.5-pro")
        || model_lower == "mimo-v2.5"
        || model_lower.contains("mimo-v2-pro");
    let is_256k_text =
        model_lower.contains("mimo-v2-omni") || model_lower.contains("mimo-v2-flash");
    let is_mimo_text = !is_tts && (is_1m_text || is_256k_text);

    let context_window = if is_tts {
        8_000
    } else if is_1m_text {
        crate::models::XIAOMIMIMO_1M_CONTEXT_WINDOW_TOKENS
    } else if is_256k_text {
        crate::models::XIAOMIMIMO_256K_CONTEXT_WINDOW_TOKENS
    } else {
        crate::models::context_window_for_model(&model_lower)
            .unwrap_or(crate::models::DEFAULT_CONTEXT_WINDOW_TOKENS)
    };

    let max_output = if is_tts {
        8_192
    } else if model_lower.contains("mimo-v2-flash") {
        65_536
    } else if is_mimo_text {
        131_072
    } else {
        4_096
    };

    let thinking_supported = is_mimo_text;

    // Xiaomi MiMo native Chat Completions reports prompt cache details under
    // `usage.prompt_tokens_details.cached_tokens`.
    let cache_telemetry_supported = matches!(provider, ApiProvider::XiaomiMiMo);

    let request_payload_mode = RequestPayloadMode::ChatCompletions;
    let deprecation = deprecation_for_model(resolved_model);

    ProviderCapability {
        provider,
        resolved_model: resolved_model.to_string(),
        context_window,
        max_output,
        thinking_supported,
        cache_telemetry_supported,
        request_payload_mode,
        deprecation: deprecation.cloned(),
    }
}

/// Canonicalize common model aliases to official Xiaomi MiMo model IDs.
#[must_use]
pub fn canonical_model_name(model: &str) -> Option<&'static str> {
    match model.trim().to_ascii_lowercase().as_str() {
        "mimo" | "xiaomimimo" | "xiaomi-mimo" | "xiaomi_mimo" | "mimo-pro" | "mimo-v25-pro"
        | "mimo-v2.5-pro" | "xiaomimimo-v4" | "xiaomimimo-v4-pro" | "xiaomimimo-v4pro" => {
            Some("mimo-v2.5-pro")
        }
        "mimo-v25" | "mimo-v2.5" => Some("mimo-v2.5"),
        "mimo-v2-pro" => Some("mimo-v2-pro"),
        "mimo-omni" | "mimo-v2-omni" => Some("mimo-v2-omni"),
        "mimo-flash"
        | "mimo-v2-flash"
        | "mimo-chat"
        | "xiaomimimo-chat"
        | "xiaomimimo-reasoner"
        | "xiaomimimo-r1"
        | "xiaomimimo-v3"
        | "xiaomimimo-v3.2"
        | "xiaomimimo-v4-flash"
        | "xiaomimimo-v4flash" => Some("mimo-v2-flash"),
        "mimo-v2-tts" => Some("mimo-v2-tts"),
        "mimo-tts" | "mimo-v25-tts" | "mimo-v2.5-tts" => Some("mimo-v2.5-tts"),
        "mimo-tts-voicedesign"
        | "mimo-voice-design"
        | "mimo-v25-tts-voicedesign"
        | "mimo-v2.5-tts-voicedesign" => Some("mimo-v2.5-tts-voicedesign"),
        "mimo-tts-voiceclone"
        | "mimo-voice-clone"
        | "mimo-v25-tts-voiceclone"
        | "mimo-v2.5-tts-voiceclone" => Some("mimo-v2.5-tts-voiceclone"),
        _ => None,
    }
}

/// Normalize a configured/runtime model name.
///
/// Accepts known aliases plus official `mimo-*` model IDs so future Xiaomi MiMo
/// releases work without code changes.
#[must_use]
pub fn normalize_model_name(model: &str) -> Option<String> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(canonical) = canonical_model_name(trimmed) {
        return Some(canonical.to_string());
    }

    let normalized = trimmed.to_ascii_lowercase();
    if !(normalized.starts_with("mimo-") || normalized.starts_with("xiaomimimo/")) {
        return None;
    }

    if normalized.chars().all(|ch| {
        ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_' | '.' | ':' | '/')
    }) {
        return Some(normalized);
    }

    None
}

// === Types ===

/// Raw retry configuration loaded from config files.
#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    pub enabled: Option<bool>,
    pub max_retries: Option<u32>,
    pub initial_delay: Option<f64>,
    pub max_delay: Option<f64>,
    pub exponential_base: Option<f64>,
}

/// UI configuration loaded from config files.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TuiConfig {
    pub alternate_screen: Option<String>,
    pub mouse_capture: Option<bool>,
    /// When true, bare Up/Down on an empty composer scrolls the transcript
    /// instead of navigating prompt history. `None` uses the platform default.
    pub composer_arrows_scroll: Option<bool>,
    /// Ordered list of footer items the user wants visible. `None` (the field
    /// missing from `config.toml`) means "use the built-in default order"; an
    /// empty `Some(vec![])` means "show nothing in the footer".
    ///
    /// Edited interactively via `/statusline`; persisted to `tui.status_items`
    /// in `~/.xiaomimimo/config.toml`.
    pub status_items: Option<Vec<StatusItem>>,
}

/// Notification delivery method (mirrors `tui::notifications::Method`).
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationMethod {
    /// Auto-detect: OSC 9 for iTerm.app / Ghostty / WezTerm; BEL otherwise.
    #[default]
    Auto,
    /// OSC 9 escape.
    Osc9,
    /// Plain BEL character.
    Bel,
    /// Disable notifications.
    Off,
}

fn default_threshold_secs() -> u64 {
    30
}

/// Desktop-notification configuration (OSC 9 / BEL on turn completion).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NotificationsConfig {
    /// Delivery method: `auto` | `osc9` | `bel` | `off`. Default: `auto`.
    #[serde(default)]
    pub method: NotificationMethod,
    /// Only notify when the turn took at least this many seconds. Default: 30.
    #[serde(default = "default_threshold_secs")]
    pub threshold_secs: u64,
    /// Include a short summary (elapsed time + turn token count) in the
    /// notification body.
    /// Default: `false`.
    #[serde(default)]
    pub include_summary: bool,
}

fn default_snapshots_enabled() -> bool {
    false
}

fn default_snapshot_max_age_days() -> u64 {
    crate::snapshot::DEFAULT_MAX_AGE.as_secs() / (24 * 60 * 60)
}

/// Workspace side-git snapshot configuration (#137).
#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotsConfig {
    /// Snapshot the workspace before and after each interactive agent turn.
    #[serde(default = "default_snapshots_enabled")]
    pub enabled: bool,
    /// Prune side-git snapshots older than this many days at session boot.
    #[serde(default = "default_snapshot_max_age_days")]
    pub max_age_days: u64,
}

impl Default for SnapshotsConfig {
    fn default() -> Self {
        Self {
            enabled: default_snapshots_enabled(),
            max_age_days: default_snapshot_max_age_days(),
        }
    }
}

impl SnapshotsConfig {
    #[must_use]
    pub fn max_age(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.max_age_days.saturating_mul(24 * 60 * 60))
    }
}

/// One configurable footer item.
///
/// Order in the user's `Vec<StatusItem>` is preserved: items in the left
/// cluster (`Mode`, `Model`, `Cost`, `Status`) render in the order given;
/// right-cluster chips (`Coherence`, `Agents`, `ReasoningReplay`, `Cache`,
/// `ContextPercent`, `GitBranch`, `LastToolElapsed`, `RateLimit`) likewise
/// honour ordering inside their cluster. The split between left and right is
/// deliberate — left holds steady identity (mode/model/cost), right holds
/// transient signals — so we route each variant to the correct side rather
/// than letting users reorder across the spacer.
///
/// Variants without a current data source (`RateLimit`, `LastToolElapsed`)
/// are intentionally exposed today so the picker is forward-compatible; they
/// render empty until the supporting fields land. Empty spans don't take
/// up footer width, so the user sees no visual artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StatusItem {
    /// "agent" / "yolo" / "plan" chip.
    Mode,
    /// Model identifier (e.g. `mimo-v2.5-pro`).
    Model,
    /// Session token usage ("tok 12.3k").
    Cost,
    /// Activity label: "ready" / "draft" / "working".
    Status,
    /// Coherence intervention label: "refreshing context" / "verifying" / "resetting plan".
    Coherence,
    /// Sub-agent count chip ("3 agents").
    Agents,
    /// Reasoning-replay token count ("rsn 12.3k").
    ReasoningReplay,
    /// Cache hit rate ("cache 73%").
    Cache,
    /// Context-window utilisation percent ("48%").
    ContextPercent,
    /// Current git branch name (placeholder until wired).
    GitBranch,
    /// Elapsed time of the most recent tool call (placeholder until wired).
    LastToolElapsed,
    /// Remaining rate-limit budget (placeholder until wired).
    RateLimit,
}

impl StatusItem {
    /// Default footer composition matching v0.6.6 behaviour exactly. Used when
    /// `tui.status_items` is missing from `config.toml` so upgraders see the
    /// same footer they had before.
    #[must_use]
    pub fn default_footer() -> Vec<StatusItem> {
        vec![
            StatusItem::Mode,
            StatusItem::Model,
            StatusItem::Cost,
            StatusItem::Status,
            StatusItem::Coherence,
            StatusItem::Agents,
            StatusItem::ReasoningReplay,
            StatusItem::Cache,
        ]
    }

    /// Stable canonical name used in TOML and the picker label.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            StatusItem::Mode => "mode",
            StatusItem::Model => "model",
            StatusItem::Cost => "cost",
            StatusItem::Status => "status",
            StatusItem::Coherence => "coherence",
            StatusItem::Agents => "agents",
            StatusItem::ReasoningReplay => "reasoning_replay",
            StatusItem::Cache => "cache",
            StatusItem::ContextPercent => "context_percent",
            StatusItem::GitBranch => "git_branch",
            StatusItem::LastToolElapsed => "last_tool_elapsed",
            StatusItem::RateLimit => "rate_limit",
        }
    }

    /// Human-readable label for the picker.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            StatusItem::Mode => "Mode",
            StatusItem::Model => "Model",
            StatusItem::Cost => "Session tokens",
            StatusItem::Status => "Activity (ready/draft/working)",
            StatusItem::Coherence => "Coherence interventions",
            StatusItem::Agents => "Sub-agents in flight",
            StatusItem::ReasoningReplay => "Reasoning replay tokens",
            StatusItem::Cache => "Prompt cache hit rate",
            StatusItem::ContextPercent => "Context window %",
            StatusItem::GitBranch => "Git branch",
            StatusItem::LastToolElapsed => "Last tool elapsed",
            StatusItem::RateLimit => "Rate-limit remaining",
        }
    }

    /// One-line hint shown beside the label so the user knows what each item
    /// surfaces without having to toggle it on first.
    #[must_use]
    pub fn hint(self) -> &'static str {
        match self {
            StatusItem::Mode => "agent · yolo · plan",
            StatusItem::Model => "the model id you'll send to",
            StatusItem::Cost => "running token total for this session",
            StatusItem::Status => "what the agent is doing right now",
            StatusItem::Coherence => "shown only when the engine intervenes",
            StatusItem::Agents => "swarm in progress",
            StatusItem::ReasoningReplay => "thinking tokens replayed each turn",
            StatusItem::Cache => "% of prompt served from cache",
            StatusItem::ContextPercent => "tokens used / model context window",
            StatusItem::GitBranch => "current branch (placeholder)",
            StatusItem::LastToolElapsed => "ms of the most recent tool call (placeholder)",
            StatusItem::RateLimit => "remaining requests in the budget (placeholder)",
        }
    }

    /// Every variant in display order — used by the picker to enumerate rows.
    #[must_use]
    pub fn all() -> &'static [StatusItem] {
        &[
            StatusItem::Mode,
            StatusItem::Model,
            StatusItem::Cost,
            StatusItem::Status,
            StatusItem::Coherence,
            StatusItem::Agents,
            StatusItem::ReasoningReplay,
            StatusItem::Cache,
            StatusItem::ContextPercent,
            StatusItem::GitBranch,
            StatusItem::LastToolElapsed,
            StatusItem::RateLimit,
        ]
    }

    /// Items that belong in the footer's left cluster (steady identity).
    #[must_use]
    pub fn is_left_cluster(self) -> bool {
        matches!(
            self,
            StatusItem::Mode | StatusItem::Model | StatusItem::Cost | StatusItem::Status
        )
    }
}

/// Resolved retry policy with defaults applied.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub enabled: bool,
    pub max_retries: u32,
    pub initial_delay: f64,
    pub max_delay: f64,
    pub exponential_base: f64,
}

/// Capacity-controller config loaded from config files/environment.
#[derive(Debug, Clone, Deserialize)]
pub struct CapacityConfig {
    pub enabled: Option<bool>,
    pub low_risk_max: Option<f64>,
    pub medium_risk_max: Option<f64>,
    pub severe_min_slack: Option<f64>,
    pub severe_violation_ratio: Option<f64>,
    pub refresh_cooldown_turns: Option<u64>,
    pub replan_cooldown_turns: Option<u64>,
    pub max_replay_per_turn: Option<usize>,
    pub min_turns_before_guardrail: Option<u64>,
    pub profile_window: Option<usize>,
    pub xiaomimimo_v3_2_chat_prior: Option<f64>,
    pub xiaomimimo_v3_2_reasoner_prior: Option<f64>,
    pub xiaomimimo_v4_pro_prior: Option<f64>,
    pub xiaomimimo_v4_flash_prior: Option<f64>,
    pub fallback_default_prior: Option<f64>,
}

impl RetryPolicy {
    /// Compute the backoff delay for a retry attempt.
    #[must_use]
    #[allow(dead_code)] // used by runtime_api; will be wired into client retry loop
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        let exponent = i32::try_from(attempt).unwrap_or(i32::MAX);
        let delay = self.initial_delay * self.exponential_base.powi(exponent);
        let delay = delay.min(self.max_delay);
        // Clamp to a sane range to guard against NaN/negative from misconfigured values
        let delay = delay.clamp(0.0, 300.0);
        std::time::Duration::from_secs_f64(delay)
    }
}

/// Context management configuration (append-only layered context with Flash seams).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContextConfig {
    /// Master enable for layered context management. Default: false while
    /// v0.7.5 audits Xiaomi MiMo prefix-cache behavior.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Verbatim window: last N turns never summarized. Default: 16.
    #[serde(default)]
    pub verbatim_window_turns: Option<usize>,
    /// Soft seam thresholds based on the active request input estimate.
    #[serde(default)]
    pub l1_threshold: Option<usize>,
    #[serde(default)]
    pub l2_threshold: Option<usize>,
    #[serde(default)]
    pub l3_threshold: Option<usize>,
    /// Hard cycle boundary. Default: 768000.
    #[serde(default)]
    pub cycle_threshold: Option<usize>,
    /// Model used for seam/briefing work. Default: "mimo-v2-flash".
    #[serde(default)]
    pub seam_model: Option<String>,
    /// Per-model threshold overrides.
    #[serde(default)]
    pub per_model: Option<HashMap<String, PerModelContextConfig>>,
}

/// Sub-agent model overrides. Keys in `models` can be role names (`worker`,
/// `explorer`, `awaiter`) or type names (`general`, `explore`, `plan`,
/// `review`, `custom`). Per-call explicit model choices still win.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SubagentsConfig {
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub worker_model: Option<String>,
    #[serde(default)]
    pub explorer_model: Option<String>,
    #[serde(default)]
    pub awaiter_model: Option<String>,
    #[serde(default)]
    pub review_model: Option<String>,
    #[serde(default)]
    pub custom_model: Option<String>,
    #[serde(default)]
    pub models: Option<HashMap<String, String>>,
}

/// Speech/TTS output defaults shared by the CLI command and model-visible tool.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SpeechConfig {
    /// Default directory for generated audio when no explicit output path is
    /// supplied. Relative paths are resolved by the caller (CLI cwd or TUI
    /// workspace); `~` is expanded during access.
    #[serde(default)]
    pub output_dir: Option<String>,
}

/// Per-model context tuning.
#[derive(Debug, Clone, Deserialize)]
pub struct PerModelContextConfig {
    #[serde(default)]
    pub l1_threshold: Option<usize>,
    #[serde(default)]
    pub l2_threshold: Option<usize>,
    #[serde(default)]
    pub l3_threshold: Option<usize>,
    #[serde(default)]
    pub cycle_threshold: Option<usize>,
}

/// Resolved CLI configuration, including defaults and environment overrides.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_text_model: Option<String>,
    /// XiaomiMiMo reasoning-effort tier: `"off" | "low" | "medium" | "high" | "max"`.
    /// Defaults to `"max"` at runtime if unset.
    pub reasoning_effort: Option<String>,
    pub tools_file: Option<String>,
    pub skills_dir: Option<String>,
    pub mcp_config_path: Option<String>,
    pub notes_path: Option<String>,
    pub memory_path: Option<String>,
    pub allow_shell: Option<bool>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub managed_config_path: Option<String>,
    pub requirements_path: Option<String>,
    pub max_subagents: Option<usize>,
    pub retry: Option<RetryConfig>,
    pub capacity: Option<CapacityConfig>,
    pub features: Option<FeaturesToml>,

    /// TUI configuration (alternate screen, etc.)
    pub tui: Option<TuiConfig>,

    /// Lifecycle hooks configuration
    #[serde(default)]
    pub hooks: Option<HooksConfig>,

    /// Provider-specific credentials and defaults shared with the `xiaomimimo` facade.
    #[serde(default)]
    pub providers: Option<ProvidersConfig>,

    /// Desktop notification settings (OSC 9 / BEL on long turn completion).
    #[serde(default)]
    pub notifications: Option<NotificationsConfig>,

    /// Per-domain network policy (#135). When absent, network tools fall back
    /// to a permissive default that mirrors pre-v0.7.0 behavior.
    #[serde(default)]
    pub network: Option<NetworkPolicyToml>,

    /// Community skill installer settings (#140). When absent, installer
    /// commands fall back to the bundled defaults
    /// ([`crate::skills::install::DEFAULT_REGISTRY_URL`] +
    /// [`crate::skills::install::DEFAULT_MAX_SIZE_BYTES`]).
    #[serde(default)]
    pub skills: Option<SkillsConfig>,

    /// Workspace side-git snapshots (#137). Defaults to disabled with 7-day
    /// retention when the table is absent. Enable explicitly in a project
    /// workspace if restore checkpoints are desired.
    #[serde(default)]
    pub snapshots: Option<SnapshotsConfig>,

    /// Post-edit LSP diagnostics injection (#136). When absent, the engine
    /// applies the defaults documented in [`LspConfigToml`].
    #[serde(default)]
    pub lsp: Option<LspConfigToml>,

    /// Append-only layered context management with Flash seam manager (#159).
    #[serde(default)]
    pub context: ContextConfig,

    /// Sub-agent model overrides.
    #[serde(default)]
    pub subagents: Option<SubagentsConfig>,

    /// Speech/TTS output defaults.
    #[serde(default)]
    pub speech: Option<SpeechConfig>,
}

/// `[skills]` table — knobs for the community-skill installer.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SkillsConfig {
    /// Curated registry index. `/skill install <name>` looks up the spec here.
    /// Defaults to [`crate::skills::install::DEFAULT_REGISTRY_URL`].
    #[serde(default)]
    pub registry_url: Option<String>,
    /// Per-skill maximum *uncompressed* size in bytes. Tarballs that exceed
    /// this limit are rejected during validation. Defaults to 5 MiB.
    #[serde(default)]
    pub max_install_size_bytes: Option<u64>,
}

impl SkillsConfig {
    /// Resolve the registry URL with the bundled default.
    #[must_use]
    pub fn registry_url(&self) -> String {
        self.registry_url
            .clone()
            .unwrap_or_else(|| crate::skills::install::DEFAULT_REGISTRY_URL.to_string())
    }

    /// Resolve the max install size with the bundled default.
    #[must_use]
    pub fn max_install_size_bytes(&self) -> u64 {
        self.max_install_size_bytes
            .unwrap_or(crate::skills::install::DEFAULT_MAX_SIZE_BYTES)
    }
}

/// `[network]` table — mirrors `xiaomimimo_config::NetworkPolicyToml` so the live
/// TUI runtime can construct a [`crate::network_policy::NetworkPolicy`]
/// without reaching into the workspace config crate. See `config.example.toml`
/// for documentation.
#[derive(Debug, Clone, Deserialize)]
pub struct NetworkPolicyToml {
    /// Decision for hosts that are not in `allow` or `deny`. One of
    /// `"allow" | "deny" | "prompt"`. Defaults to `"prompt"`.
    #[serde(default = "default_network_decision")]
    pub default: String,
    /// Hosts that are always allowed. Subdomain rules: a leading dot
    /// (`.example.com`) matches subdomains but not the apex.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Hosts that are always denied. Deny entries win over allow entries.
    #[serde(default)]
    pub deny: Vec<String>,
    /// Hosts whose DNS may legitimately resolve to proxy fake-IP ranges.
    /// Literal restricted IP URLs remain blocked.
    #[serde(default)]
    pub proxy: Vec<String>,
    /// Whether to record one audit-log line per outbound network call.
    #[serde(default = "default_network_audit")]
    pub audit: bool,
}

fn default_network_decision() -> String {
    "prompt".to_string()
}

fn default_network_audit() -> bool {
    true
}

impl Default for NetworkPolicyToml {
    fn default() -> Self {
        Self {
            default: default_network_decision(),
            allow: Vec::new(),
            deny: Vec::new(),
            proxy: Vec::new(),
            audit: default_network_audit(),
        }
    }
}

impl NetworkPolicyToml {
    /// Build a runtime [`crate::network_policy::NetworkPolicy`] from the
    /// on-disk schema.
    #[must_use]
    pub fn into_runtime(self) -> crate::network_policy::NetworkPolicy {
        crate::network_policy::NetworkPolicy {
            default: crate::network_policy::Decision::parse(&self.default).into(),
            allow: self.allow,
            deny: self.deny,
            proxy: self.proxy,
            audit: self.audit,
        }
    }
}

/// `[lsp]` table — mirrors [`crate::lsp::LspConfig`]. Documented in
/// `config.example.toml`. When omitted, defaults from `LspConfig::default()`
/// apply (enabled, 5 s poll, 20 diagnostics/file, errors only, no overrides).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LspConfigToml {
    /// Master switch. Defaults to `true`.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// How long to wait for the LSP server to publish diagnostics after a
    /// `didOpen`/`didChange`. Defaults to 5000 ms.
    #[serde(default)]
    pub poll_after_edit_ms: Option<u64>,
    /// Cap on diagnostics surfaced per file. Defaults to 20.
    #[serde(default)]
    pub max_diagnostics_per_file: Option<usize>,
    /// Whether to surface warnings in addition to errors. Defaults to `false`.
    #[serde(default)]
    pub include_warnings: Option<bool>,
    /// Optional override for the `Language -> [cmd, ...args]` table. Keys
    /// are language slugs (`"rust"`, `"go"`, etc.).
    #[serde(default)]
    pub servers: Option<HashMap<String, Vec<String>>>,
}

impl LspConfigToml {
    /// Build a runtime [`crate::lsp::LspConfig`] from the on-disk schema,
    /// falling back to defaults for any unset fields.
    #[must_use]
    pub fn into_runtime(self) -> crate::lsp::LspConfig {
        let defaults = crate::lsp::LspConfig::default();
        crate::lsp::LspConfig {
            enabled: self.enabled.unwrap_or(defaults.enabled),
            poll_after_edit_ms: self
                .poll_after_edit_ms
                .unwrap_or(defaults.poll_after_edit_ms),
            max_diagnostics_per_file: self
                .max_diagnostics_per_file
                .unwrap_or(defaults.max_diagnostics_per_file),
            include_warnings: self.include_warnings.unwrap_or(defaults.include_warnings),
            servers: self.servers.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub xiaomimimo: ProviderConfig,
    #[serde(default)]
    pub nvidia_nim: ProviderConfig,
    #[serde(default)]
    pub openrouter: ProviderConfig,
    #[serde(default)]
    pub novita: ProviderConfig,
    #[serde(default)]
    pub fireworks: ProviderConfig,
    #[serde(default)]
    pub sglang: ProviderConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ConfigFile {
    #[serde(flatten)]
    base: Config,
    profiles: Option<HashMap<String, Config>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RequirementsFile {
    #[serde(default)]
    allowed_approval_policies: Vec<String>,
    #[serde(default)]
    allowed_sandbox_modes: Vec<String>,
}

// === Config Loading ===

impl Config {
    /// Load configuration from disk and merge with environment overrides.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use crate::config::Config;
    /// let config = Config::load(None, None)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn load(path: Option<PathBuf>, profile: Option<&str>) -> Result<Self> {
        let path = resolve_load_config_path(path);
        let mut config = if let Some(path) = path.as_ref() {
            if path.exists() {
                let contents = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?;
                let parsed: ConfigFile = toml::from_str(&contents)
                    .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
                apply_profile(parsed, profile)?
            } else {
                Config::default()
            }
        } else {
            Config::default()
        };

        apply_env_overrides(&mut config);
        apply_managed_overrides(&mut config)?;
        apply_requirements(&mut config)?;
        normalize_model_config(&mut config);
        config.validate()?;
        Ok(config)
    }

    /// Validate that critical config fields are present.
    pub fn validate(&self) -> Result<()> {
        if let Some(provider) = self.provider.as_deref()
            && ApiProvider::parse(provider).is_none()
        {
            anyhow::bail!(
                "Invalid provider '{provider}': expected xiaomimimo, nvidia-nim, openrouter, novita, fireworks, or sglang."
            );
        }
        if let Some(ref key) = self.api_key
            && key.trim().is_empty()
        {
            anyhow::bail!("api_key cannot be empty string");
        }
        if let Some(features) = &self.features {
            for key in features.entries.keys() {
                if !is_known_feature_key(key) {
                    anyhow::bail!("Unknown feature flag: {key}");
                }
            }
        }
        if let Some(model) = self.default_text_model.as_deref()
            && normalize_model_name(model).is_none()
        {
            anyhow::bail!(
                "Invalid default_text_model '{model}': expected a Xiaomi MiMo model ID (for example: mimo-v2.5-pro, mimo-v2.5, mimo-v2-flash)."
            );
        }
        if let Some(policy) = self.approval_policy.as_deref() {
            let normalized = policy.trim().to_ascii_lowercase();
            if !matches!(
                normalized.as_str(),
                "on-request" | "untrusted" | "never" | "auto" | "suggest"
            ) {
                anyhow::bail!(
                    "Invalid approval_policy '{policy}': expected on-request, untrusted, never, auto, or suggest."
                );
            }
        }
        if let Some(mode) = self.sandbox_mode.as_deref() {
            let normalized = mode.trim().to_ascii_lowercase();
            if !matches!(
                normalized.as_str(),
                "read-only" | "workspace-write" | "danger-full-access" | "external-sandbox"
            ) {
                anyhow::bail!(
                    "Invalid sandbox_mode '{mode}': expected read-only, workspace-write, danger-full-access, or external-sandbox."
                );
            }
        }
        if let Some(tui) = &self.tui
            && let Some(mode) = tui.alternate_screen.as_deref()
        {
            let mode = mode.to_ascii_lowercase();
            if !matches!(mode.as_str(), "auto" | "always" | "never") {
                anyhow::bail!(
                    "Invalid tui.alternate_screen '{mode}': expected auto, always, or never."
                );
            }
        }
        if let Some(capacity) = &self.capacity {
            if let Some(v) = capacity.low_risk_max
                && !(0.0..=1.0).contains(&v)
            {
                anyhow::bail!(
                    "Invalid capacity.low_risk_max '{v}': expected a value in [0.0, 1.0]."
                );
            }
            if let Some(v) = capacity.medium_risk_max
                && !(0.0..=1.0).contains(&v)
            {
                anyhow::bail!(
                    "Invalid capacity.medium_risk_max '{v}': expected a value in [0.0, 1.0]."
                );
            }
            if let (Some(low), Some(medium)) = (capacity.low_risk_max, capacity.medium_risk_max)
                && low > medium
            {
                anyhow::bail!(
                    "Invalid capacity thresholds: low_risk_max ({low}) must be <= medium_risk_max ({medium})."
                );
            }
            if let Some(v) = capacity.severe_violation_ratio
                && !(0.0..=1.0).contains(&v)
            {
                anyhow::bail!(
                    "Invalid capacity.severe_violation_ratio '{v}': expected a value in [0.0, 1.0]."
                );
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn api_provider(&self) -> ApiProvider {
        self.provider
            .as_deref()
            .and_then(ApiProvider::parse)
            .unwrap_or_else(|| {
                self.base_url
                    .as_deref()
                    .filter(|base| base.contains("integrate.api.nvidia.com"))
                    .map(|_| ApiProvider::NvidiaNim)
                    .unwrap_or(ApiProvider::XiaomiMiMo)
            })
    }

    fn provider_config_for(&self, provider: ApiProvider) -> Option<&ProviderConfig> {
        let providers = self.providers.as_ref()?;
        Some(match provider {
            ApiProvider::XiaomiMiMo => &providers.xiaomimimo,
            ApiProvider::NvidiaNim => &providers.nvidia_nim,
            ApiProvider::Openrouter => &providers.openrouter,
            ApiProvider::Novita => &providers.novita,
            ApiProvider::Fireworks => &providers.fireworks,
            ApiProvider::Sglang => &providers.sglang,
        })
    }

    fn provider_config(&self) -> Option<&ProviderConfig> {
        self.provider_config_for(self.api_provider())
    }

    #[must_use]
    pub fn default_model(&self) -> String {
        let provider = self.api_provider();
        if let Some(model) = self
            .provider_config()
            .and_then(|provider| provider.model.as_deref())
            .map(str::trim)
            .filter(|model| !model.is_empty())
        {
            if let Some(normalized) = normalize_model_for_provider(provider, model) {
                return normalized;
            }
            if !matches!(provider, ApiProvider::XiaomiMiMo) {
                return model.to_string();
            }
        }
        if let Some(model) = self.default_text_model.as_deref()
            && let Some(normalized) = normalize_model_name(model)
        {
            return model_for_provider(provider, normalized);
        }

        match provider {
            ApiProvider::XiaomiMiMo => DEFAULT_TEXT_MODEL,
            ApiProvider::NvidiaNim => DEFAULT_NVIDIA_NIM_MODEL,
            ApiProvider::Openrouter => DEFAULT_OPENROUTER_MODEL,
            ApiProvider::Novita => DEFAULT_NOVITA_MODEL,
            ApiProvider::Fireworks => DEFAULT_FIREWORKS_MODEL,
            ApiProvider::Sglang => DEFAULT_SGLANG_MODEL,
        }
        .to_string()
    }

    /// Return the configured API base URL (normalized).
    #[must_use]
    pub fn xiaomimimo_base_url(&self) -> String {
        let provider = self.api_provider();
        let provider_base = self
            .provider_config_for(provider)
            .and_then(|provider| provider.base_url.clone());
        // Root `base_url` is the legacy XiaomiMiMo field; only NvidiaNim has a
        // back-compat sniff (integrate.api.nvidia.com). OpenRouter / Novita
        // were added in v0.6.7 and require explicit `[providers.<name>]`
        // entries or the corresponding `*_BASE_URL` env var.
        let root_base = match provider {
            ApiProvider::XiaomiMiMo => self.base_url.clone(),
            ApiProvider::NvidiaNim => self
                .base_url
                .as_ref()
                .filter(|base| base.contains("integrate.api.nvidia.com"))
                .cloned(),
            ApiProvider::Openrouter
            | ApiProvider::Novita
            | ApiProvider::Fireworks
            | ApiProvider::Sglang => None,
        };
        let base = provider_base.or(root_base).unwrap_or_else(|| {
            match provider {
                ApiProvider::XiaomiMiMo => "https://token-plan-cn.xiaomimimo.com/v1",
                ApiProvider::NvidiaNim => DEFAULT_NVIDIA_NIM_BASE_URL,
                ApiProvider::Openrouter => DEFAULT_OPENROUTER_BASE_URL,
                ApiProvider::Novita => DEFAULT_NOVITA_BASE_URL,
                ApiProvider::Fireworks => DEFAULT_FIREWORKS_BASE_URL,
                ApiProvider::Sglang => DEFAULT_SGLANG_BASE_URL,
            }
            .to_string()
        });
        normalize_base_url(&base)
    }

    /// Read the API key.
    ///
    /// Precedence: **OS keyring → environment → config file**. The
    /// keyring + env layers are collapsed by [`xiaomimimo_secrets::Secrets::resolve`];
    /// the config-file fallback is preserved here for users who haven't
    /// run `xiaomimimo auth migrate` yet.
    pub fn xiaomimimo_api_key(&self) -> Result<String> {
        let provider = self.api_provider();
        let slot = match provider {
            ApiProvider::XiaomiMiMo => "xiaomimimo",
            ApiProvider::NvidiaNim => "nvidia-nim",
            ApiProvider::Openrouter => "openrouter",
            ApiProvider::Novita => "novita",
            ApiProvider::Fireworks => "fireworks",
            ApiProvider::Sglang => "sglang",
        };

        // 1. OS keyring + 2. environment variables (handled by Secrets).
        let secrets = xiaomimimo_secrets::Secrets::auto_detect();
        if let Some(value) = secrets.resolve(slot)
            && !value.trim().is_empty()
        {
            return Ok(value);
        }

        // 3. config file (provider-scoped slot).
        if let Some(configured) = self
            .provider_config_for(provider)
            .and_then(|provider| provider.api_key.clone())
            && !configured.trim().is_empty()
        {
            tracing::warn!(
                "[providers.{slot}] api_key in config.toml is deprecated; \
                 run 'xiaomimimo auth set --provider {slot}' to move it to the OS keyring"
            );
            return Ok(configured);
        }

        // 4. legacy root `api_key` (xiaomimimo only).
        if let Some(configured) = self.api_key.clone()
            && !configured.trim().is_empty()
            && configured != API_KEYRING_SENTINEL
        {
            tracing::warn!(
                "api_key in config.toml is deprecated; run 'xiaomimimo auth migrate' to move it to the OS keyring"
            );
            return Ok(configured);
        }

        match provider {
            ApiProvider::XiaomiMiMo => anyhow::bail!(
                "XiaomiMiMo API key not found. Set it using one of these methods:\n\
                 1. Run 'xiaomimimo auth set --provider xiaomimimo' to save it in the OS keyring (recommended)\n\
                 2. Set XIAOMIMIMO_API_KEY environment variable\n\
                 3. Add 'api_key = \"your-key\"' to ~/.xiaomimimo/config.toml (deprecated)"
            ),
            ApiProvider::NvidiaNim => anyhow::bail!(
                "NVIDIA NIM API key not found. Run 'xiaomimimo auth set --provider nvidia-nim', \
                 set NVIDIA_API_KEY/NVIDIA_NIM_API_KEY, or save api_key in ~/.xiaomimimo/config.toml \
                 with provider = \"nvidia-nim\"."
            ),
            ApiProvider::Openrouter => anyhow::bail!(
                "OpenRouter API key not found. Run 'xiaomimimo auth set --provider openrouter', \
                 set OPENROUTER_API_KEY, or add [providers.openrouter] api_key in ~/.xiaomimimo/config.toml."
            ),
            ApiProvider::Novita => anyhow::bail!(
                "Novita API key not found. Run 'xiaomimimo auth set --provider novita', \
                 set NOVITA_API_KEY, or add [providers.novita] api_key in ~/.xiaomimimo/config.toml."
            ),
            ApiProvider::Fireworks => anyhow::bail!(
                "Fireworks AI API key not found. Run 'xiaomimimo auth set --provider fireworks', \
                 set FIREWORKS_API_KEY, or add [providers.fireworks] api_key in ~/.xiaomimimo/config.toml."
            ),
            // Self-hosted SGLang deployments commonly run without auth on
            // localhost. Return an empty key and let the client omit the
            // Authorization header.
            ApiProvider::Sglang => Ok(String::new()),
        }
    }

    /// Resolve the skills directory path.
    #[must_use]
    pub fn skills_dir(&self) -> PathBuf {
        self.skills_dir
            .as_deref()
            .map(expand_path)
            .or_else(default_skills_dir)
            .unwrap_or_else(|| PathBuf::from("./skills"))
    }

    /// Resolve the MCP config path.
    #[must_use]
    pub fn mcp_config_path(&self) -> PathBuf {
        self.mcp_config_path
            .as_deref()
            .map(expand_path)
            .or_else(default_mcp_config_path)
            .unwrap_or_else(|| PathBuf::from("./mcp.json"))
    }

    /// Resolve the notes file path.
    #[must_use]
    pub fn notes_path(&self) -> PathBuf {
        self.notes_path
            .as_deref()
            .map(expand_path)
            .or_else(default_notes_path)
            .unwrap_or_else(|| PathBuf::from("./notes.txt"))
    }

    /// Resolve the memory file path.
    #[must_use]
    pub fn memory_path(&self) -> PathBuf {
        self.memory_path
            .as_deref()
            .map(expand_path)
            .or_else(default_memory_path)
            .unwrap_or_else(|| PathBuf::from("./memory.md"))
    }

    /// Resolve the configured speech/TTS output directory, if any.
    #[must_use]
    pub fn speech_output_dir(&self) -> Option<PathBuf> {
        self.speech
            .as_ref()
            .and_then(|speech| speech.output_dir.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(expand_path)
    }

    /// Return whether shell execution is allowed.
    #[must_use]
    pub fn allow_shell(&self) -> bool {
        self.allow_shell.unwrap_or(true)
    }

    /// Return the maximum number of concurrent sub-agents.
    #[must_use]
    pub fn max_subagents(&self) -> usize {
        self.max_subagents
            .unwrap_or(DEFAULT_MAX_SUBAGENTS)
            .clamp(1, MAX_SUBAGENTS)
    }

    /// Raw sub-agent model override map. Values are validated at spawn time
    /// so an invalid role/type model fails before any partial swarm spawn.
    #[must_use]
    pub fn subagent_model_overrides(&self) -> HashMap<String, String> {
        let mut overrides = HashMap::new();
        let Some(cfg) = self.subagents.as_ref() else {
            return overrides;
        };

        let mut insert = |key: &str, value: &Option<String>| {
            if let Some(model) = value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                overrides.insert(key.to_string(), model.to_string());
            }
        };
        insert("default", &cfg.default_model);
        insert("worker", &cfg.worker_model);
        insert("general", &cfg.worker_model);
        insert("explorer", &cfg.explorer_model);
        insert("explore", &cfg.explorer_model);
        insert("awaiter", &cfg.awaiter_model);
        insert("plan", &cfg.awaiter_model);
        insert("review", &cfg.review_model);
        insert("custom", &cfg.custom_model);

        if let Some(models) = cfg.models.as_ref() {
            for (key, model) in models {
                let key = key.trim();
                let model = model.trim();
                if !key.is_empty() && !model.is_empty() {
                    overrides.insert(key.to_ascii_lowercase(), model.to_string());
                }
            }
        }

        overrides
    }

    /// Return the configured XiaomiMiMo reasoning-effort tier, if any.
    #[must_use]
    pub fn reasoning_effort(&self) -> Option<&str> {
        self.reasoning_effort.as_deref()
    }

    /// Get hooks configuration, returning default if not configured.
    pub fn hooks_config(&self) -> HooksConfig {
        self.hooks.clone().unwrap_or_default()
    }

    /// Resolve the notifications configuration with defaults applied.
    #[must_use]
    pub fn notifications_config(&self) -> NotificationsConfig {
        self.notifications.clone().unwrap_or_default()
    }

    /// Resolve workspace side-git snapshot settings with defaults applied.
    #[must_use]
    pub fn snapshots_config(&self) -> SnapshotsConfig {
        self.snapshots.clone().unwrap_or_default()
    }

    /// Resolve enabled features from defaults and config entries.
    #[must_use]
    pub fn features(&self) -> Features {
        let mut features = Features::with_defaults();
        if let Some(table) = &self.features {
            features.apply_map(&table.entries);
        }
        features
    }

    /// Override a feature flag in memory (used by CLI overrides).
    pub fn set_feature(&mut self, key: &str, enabled: bool) -> Result<()> {
        if !is_known_feature_key(key) {
            anyhow::bail!("Unknown feature flag: {key}");
        }
        let table = self.features.get_or_insert_with(FeaturesToml::default);
        table.entries.insert(key.to_string(), enabled);
        Ok(())
    }

    /// Resolve the effective retry policy with defaults applied.
    #[must_use]
    pub fn retry_policy(&self) -> RetryPolicy {
        let defaults = RetryPolicy {
            enabled: true,
            max_retries: 3,
            initial_delay: 1.0,
            max_delay: 60.0,
            exponential_base: 2.0,
        };

        let Some(cfg) = &self.retry else {
            return defaults;
        };

        RetryPolicy {
            enabled: cfg.enabled.unwrap_or(defaults.enabled),
            max_retries: cfg.max_retries.unwrap_or(defaults.max_retries),
            initial_delay: cfg.initial_delay.unwrap_or(defaults.initial_delay),
            max_delay: cfg.max_delay.unwrap_or(defaults.max_delay),
            exponential_base: cfg.exponential_base.unwrap_or(defaults.exponential_base),
        }
    }
}

// === Defaults ===

fn default_config_path() -> Option<PathBuf> {
    env_config_path().or_else(home_config_path)
}

fn effective_home_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("HOME") {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }

    if let Some(path) = std::env::var_os("USERPROFILE") {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }

    #[cfg(windows)]
    {
        if let (Some(drive), Some(homepath)) =
            (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH"))
        {
            let mut path = PathBuf::from(drive);
            path.push(homepath);
            if !path.as_os_str().is_empty() {
                return Some(path);
            }
        }
    }

    dirs::home_dir()
}

fn home_config_path() -> Option<PathBuf> {
    effective_home_dir().map(|home| home.join(".xiaomimimo").join("config.toml"))
}

fn env_config_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("XIAOMIMIMO_CONFIG_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(expand_path_with_config_env_home(trimmed, None));
        }
    }
    None
}

fn expand_pathbuf(path: PathBuf) -> PathBuf {
    if let Some(raw) = path.to_str() {
        return expand_path(raw);
    }
    path
}

fn resolve_load_config_path(path: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = path {
        return Some(expand_pathbuf(path));
    }

    if let Some(path) = env_config_path() {
        if path.exists() {
            return Some(path);
        }

        if let Some(home_path) = home_config_path()
            && home_path.exists()
        {
            return Some(home_path);
        }

        return Some(path);
    }

    home_config_path()
}

fn default_managed_config_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        Some(PathBuf::from("/etc/xiaomimimo/managed_config.toml"))
    }
    #[cfg(not(unix))]
    {
        effective_home_dir().map(|home| home.join(".xiaomimimo").join("managed_config.toml"))
    }
}

fn default_requirements_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        Some(PathBuf::from("/etc/xiaomimimo/requirements.toml"))
    }
    #[cfg(not(unix))]
    {
        effective_home_dir().map(|home| home.join(".xiaomimimo").join("requirements.toml"))
    }
}

pub(crate) fn expand_path(path: &str) -> PathBuf {
    expand_path_with_config_env_home(path, env_home_from_config_path())
}

fn expand_path_with_config_env_home(path: &str, config_env_home: Option<PathBuf>) -> PathBuf {
    if let Some(stripped) = path.strip_prefix('~')
        && (stripped.is_empty() || stripped.starts_with('/') || stripped.starts_with('\\'))
        && let Some(mut home) = config_env_home
    {
        let suffix = stripped.trim_start_matches(['/', '\\']);
        if !suffix.is_empty() {
            home.push(suffix);
        }
        return home;
    }

    if let Some(stripped) = path.strip_prefix('~')
        && (stripped.is_empty() || stripped.starts_with('/') || stripped.starts_with('\\'))
        && let Some(mut home) = effective_home_dir()
    {
        let suffix = stripped.trim_start_matches(['/', '\\']);
        if !suffix.is_empty() {
            home.push(suffix);
        }
        return home;
    }

    let expanded = shellexpand::tilde(path);
    PathBuf::from(expanded.as_ref())
}

fn env_home_from_config_path() -> Option<PathBuf> {
    std::env::var("XIAOMIMIMO_CONFIG_PATH")
        .ok()
        .and_then(|config_path| home_from_config_path(&config_path))
}

fn home_from_config_path(config_path: &str) -> Option<PathBuf> {
    let trimmed = config_path.trim();
    if trimmed.starts_with('~') {
        return None;
    }
    Path::new(trimmed)
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

fn default_skills_dir() -> Option<PathBuf> {
    effective_home_dir().map(|home| home.join(".xiaomimimo").join("skills"))
}

fn default_mcp_config_path() -> Option<PathBuf> {
    effective_home_dir().map(|home| home.join(".xiaomimimo").join("mcp.json"))
}

fn default_notes_path() -> Option<PathBuf> {
    effective_home_dir().map(|home| home.join(".xiaomimimo").join("notes.txt"))
}

fn default_memory_path() -> Option<PathBuf> {
    effective_home_dir().map(|home| home.join(".xiaomimimo").join("memory.md"))
}

// === Environment Overrides ===

fn apply_env_overrides(config: &mut Config) {
    if let Ok(value) = std::env::var("XIAOMIMIMO_PROVIDER") {
        config.provider = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_API_KEY")
        && !value.trim().is_empty()
    {
        config.api_key = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_BASE_URL") {
        match config.api_provider() {
            ApiProvider::XiaomiMiMo => {
                config.base_url = Some(value);
            }
            ApiProvider::NvidiaNim => {
                config
                    .providers
                    .get_or_insert_with(ProvidersConfig::default)
                    .nvidia_nim
                    .base_url = Some(value);
            }
            ApiProvider::Openrouter => {
                config
                    .providers
                    .get_or_insert_with(ProvidersConfig::default)
                    .openrouter
                    .base_url = Some(value);
            }
            ApiProvider::Novita => {
                config
                    .providers
                    .get_or_insert_with(ProvidersConfig::default)
                    .novita
                    .base_url = Some(value);
            }
            ApiProvider::Fireworks => {
                config
                    .providers
                    .get_or_insert_with(ProvidersConfig::default)
                    .fireworks
                    .base_url = Some(value);
            }
            ApiProvider::Sglang => {
                config
                    .providers
                    .get_or_insert_with(ProvidersConfig::default)
                    .sglang
                    .base_url = Some(value);
            }
        }
    }
    if matches!(config.api_provider(), ApiProvider::NvidiaNim)
        && let Ok(value) = std::env::var("NVIDIA_NIM_BASE_URL")
            .or_else(|_| std::env::var("NIM_BASE_URL"))
            .or_else(|_| std::env::var("NVIDIA_BASE_URL"))
    {
        config
            .providers
            .get_or_insert_with(ProvidersConfig::default)
            .nvidia_nim
            .base_url = Some(value);
    }
    // OpenRouter / Novita are scoped only on their own provider entry — the
    // legacy root `base_url` keeps XiaomiMiMo-only semantics.
    if matches!(config.api_provider(), ApiProvider::Openrouter)
        && let Ok(value) = std::env::var("OPENROUTER_BASE_URL")
        && !value.trim().is_empty()
    {
        config
            .providers
            .get_or_insert_with(ProvidersConfig::default)
            .openrouter
            .base_url = Some(value);
    }
    if matches!(config.api_provider(), ApiProvider::Novita)
        && let Ok(value) = std::env::var("NOVITA_BASE_URL")
        && !value.trim().is_empty()
    {
        config
            .providers
            .get_or_insert_with(ProvidersConfig::default)
            .novita
            .base_url = Some(value);
    }
    if matches!(config.api_provider(), ApiProvider::Fireworks)
        && let Ok(value) = std::env::var("FIREWORKS_BASE_URL")
        && !value.trim().is_empty()
    {
        config
            .providers
            .get_or_insert_with(ProvidersConfig::default)
            .fireworks
            .base_url = Some(value);
    }
    if matches!(config.api_provider(), ApiProvider::Sglang)
        && let Ok(value) = std::env::var("SGLANG_BASE_URL")
        && !value.trim().is_empty()
    {
        config
            .providers
            .get_or_insert_with(ProvidersConfig::default)
            .sglang
            .base_url = Some(value);
    }
    if matches!(config.api_provider(), ApiProvider::Sglang)
        && let Ok(value) = std::env::var("SGLANG_MODEL")
    {
        config
            .providers
            .get_or_insert_with(ProvidersConfig::default)
            .sglang
            .model = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_MODEL")
        .or_else(|_| std::env::var("XIAOMIMIMO_DEFAULT_TEXT_MODEL"))
    {
        config.default_text_model = Some(value);
    }
    if matches!(config.api_provider(), ApiProvider::NvidiaNim)
        && let Ok(value) = std::env::var("NVIDIA_NIM_MODEL")
    {
        config
            .providers
            .get_or_insert_with(ProvidersConfig::default)
            .nvidia_nim
            .model = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_SKILLS_DIR") {
        config.skills_dir = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_MCP_CONFIG") {
        config.mcp_config_path = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_NOTES_PATH") {
        config.notes_path = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_MEMORY_PATH") {
        config.memory_path = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_SPEECH_OUTPUT_DIR")
        && !value.trim().is_empty()
    {
        config
            .speech
            .get_or_insert_with(SpeechConfig::default)
            .output_dir = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_ALLOW_SHELL") {
        config.allow_shell = Some(value == "1" || value.eq_ignore_ascii_case("true"));
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_APPROVAL_POLICY") {
        config.approval_policy = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_SANDBOX_MODE") {
        config.sandbox_mode = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_MANAGED_CONFIG_PATH") {
        config.managed_config_path = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_REQUIREMENTS_PATH") {
        config.requirements_path = Some(value);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_MAX_SUBAGENTS")
        && let Ok(parsed) = value.parse::<usize>()
    {
        config.max_subagents = Some(parsed.clamp(1, MAX_SUBAGENTS));
    }

    let capacity = config.capacity.get_or_insert(CapacityConfig {
        enabled: None,
        low_risk_max: None,
        medium_risk_max: None,
        severe_min_slack: None,
        severe_violation_ratio: None,
        refresh_cooldown_turns: None,
        replan_cooldown_turns: None,
        max_replay_per_turn: None,
        min_turns_before_guardrail: None,
        profile_window: None,
        xiaomimimo_v3_2_chat_prior: None,
        xiaomimimo_v3_2_reasoner_prior: None,
        xiaomimimo_v4_pro_prior: None,
        xiaomimimo_v4_flash_prior: None,
        fallback_default_prior: None,
    });

    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_ENABLED") {
        let val = value.trim().to_ascii_lowercase();
        capacity.enabled = Some(matches!(val.as_str(), "1" | "true" | "yes" | "on"));
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_LOW_RISK_MAX")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.low_risk_max = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_MEDIUM_RISK_MAX")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.medium_risk_max = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_SEVERE_MIN_SLACK")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.severe_min_slack = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_SEVERE_VIOLATION_RATIO")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.severe_violation_ratio = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_REFRESH_COOLDOWN_TURNS")
        && let Ok(parsed) = value.parse::<u64>()
    {
        capacity.refresh_cooldown_turns = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_REPLAN_COOLDOWN_TURNS")
        && let Ok(parsed) = value.parse::<u64>()
    {
        capacity.replan_cooldown_turns = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_MAX_REPLAY_PER_TURN")
        && let Ok(parsed) = value.parse::<usize>()
    {
        capacity.max_replay_per_turn = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_MIN_TURNS_BEFORE_GUARDRAIL")
        && let Ok(parsed) = value.parse::<u64>()
    {
        capacity.min_turns_before_guardrail = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_PROFILE_WINDOW")
        && let Ok(parsed) = value.parse::<usize>()
    {
        capacity.profile_window = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_PRIOR_CHAT")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.xiaomimimo_v3_2_chat_prior = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_PRIOR_REASONER")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.xiaomimimo_v3_2_reasoner_prior = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_PRIOR_PRO")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.xiaomimimo_v4_pro_prior = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_PRIOR_FLASH")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.xiaomimimo_v4_flash_prior = Some(parsed);
    }
    if let Ok(value) = std::env::var("XIAOMIMIMO_CAPACITY_PRIOR_FALLBACK")
        && let Ok(parsed) = value.parse::<f64>()
    {
        capacity.fallback_default_prior = Some(parsed);
    }

    if config.capacity.as_ref().is_some_and(|c| {
        c.enabled.is_none()
            && c.low_risk_max.is_none()
            && c.medium_risk_max.is_none()
            && c.severe_min_slack.is_none()
            && c.severe_violation_ratio.is_none()
            && c.refresh_cooldown_turns.is_none()
            && c.replan_cooldown_turns.is_none()
            && c.max_replay_per_turn.is_none()
            && c.min_turns_before_guardrail.is_none()
            && c.profile_window.is_none()
            && c.xiaomimimo_v3_2_chat_prior.is_none()
            && c.xiaomimimo_v3_2_reasoner_prior.is_none()
            && c.xiaomimimo_v4_pro_prior.is_none()
            && c.xiaomimimo_v4_flash_prior.is_none()
            && c.fallback_default_prior.is_none()
    }) {
        config.capacity = None;
    }
}

fn normalize_model_config(config: &mut Config) {
    if let Some(model) = config.default_text_model.as_deref()
        && let Some(normalized) = normalize_model_for_provider(config.api_provider(), model)
    {
        config.default_text_model = Some(normalized);
    }

    if let Some(providers) = config.providers.as_mut() {
        if let Some(model) = providers.xiaomimimo.model.as_deref()
            && let Some(normalized) = normalize_model_for_provider(ApiProvider::XiaomiMiMo, model)
        {
            providers.xiaomimimo.model = Some(normalized);
        }
        normalize_provider_entry_model(ApiProvider::NvidiaNim, &mut providers.nvidia_nim.model);
        normalize_provider_entry_model(ApiProvider::Openrouter, &mut providers.openrouter.model);
        normalize_provider_entry_model(ApiProvider::Novita, &mut providers.novita.model);
        normalize_provider_entry_model(ApiProvider::Fireworks, &mut providers.fireworks.model);
        normalize_provider_entry_model(ApiProvider::Sglang, &mut providers.sglang.model);
    }
}

fn normalize_provider_entry_model(provider: ApiProvider, model: &mut Option<String>) {
    let Some(current) = model.as_deref().map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if let Some(normalized) = normalize_model_for_provider(provider, current) {
        *model = Some(normalized);
    } else if !matches!(provider, ApiProvider::XiaomiMiMo) {
        *model = Some(current.to_string());
    }
}

fn normalize_model_for_provider(provider: ApiProvider, model: &str) -> Option<String> {
    normalize_model_name(model).map(|normalized| model_for_provider(provider, normalized))
}

fn model_for_provider(provider: ApiProvider, normalized: String) -> String {
    match (provider, normalized.as_str()) {
        (ApiProvider::NvidiaNim, "mimo-v2.5-pro") => DEFAULT_NVIDIA_NIM_MODEL.to_string(),
        (ApiProvider::NvidiaNim, "mimo-v2-flash") => DEFAULT_NVIDIA_NIM_FLASH_MODEL.to_string(),
        (ApiProvider::Openrouter, "mimo-v2.5-pro") => DEFAULT_OPENROUTER_MODEL.to_string(),
        (ApiProvider::Openrouter, "mimo-v2-flash") => DEFAULT_OPENROUTER_FLASH_MODEL.to_string(),
        (ApiProvider::Novita, "mimo-v2.5-pro") => DEFAULT_NOVITA_MODEL.to_string(),
        (ApiProvider::Novita, "mimo-v2-flash") => DEFAULT_NOVITA_FLASH_MODEL.to_string(),
        (ApiProvider::Fireworks, "mimo-v2.5-pro") => DEFAULT_FIREWORKS_MODEL.to_string(),
        (ApiProvider::Fireworks, "mimo-v2-flash") => DEFAULT_SGLANG_FLASH_MODEL.to_string(),
        (ApiProvider::Sglang, "mimo-v2.5-pro") => DEFAULT_SGLANG_MODEL.to_string(),
        (ApiProvider::Sglang, "mimo-v2-flash") => DEFAULT_SGLANG_FLASH_MODEL.to_string(),
        _ => normalized,
    }
}

fn normalize_base_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    let xiaomimimo_domains = ["token-plan-cn.xiaomimimo.com"];
    if xiaomimimo_domains
        .iter()
        .any(|domain| trimmed.contains(domain))
    {
        return trimmed.trim_end_matches("/v1").to_string();
    }
    trimmed.to_string()
}

fn apply_profile(config: ConfigFile, profile: Option<&str>) -> Result<Config> {
    if let Some(profile_name) = profile {
        let profiles = config.profiles.as_ref();
        match profiles.and_then(|profiles| profiles.get(profile_name)) {
            Some(override_cfg) => Ok(merge_config(config.base, override_cfg.clone())),
            None => {
                let available = profiles
                    .map(|profiles| {
                        let mut keys = profiles.keys().cloned().collect::<Vec<_>>();
                        keys.sort();
                        if keys.is_empty() {
                            "none".to_string()
                        } else {
                            keys.join(", ")
                        }
                    })
                    .unwrap_or_else(|| "none".to_string());
                anyhow::bail!(
                    "Profile '{}' not found. Available profiles: {}",
                    profile_name,
                    available
                )
            }
        }
    } else {
        Ok(config.base)
    }
}

fn merge_config(base: Config, override_cfg: Config) -> Config {
    Config {
        provider: override_cfg.provider.or(base.provider),
        api_key: override_cfg.api_key.or(base.api_key),
        base_url: override_cfg.base_url.or(base.base_url),
        default_text_model: override_cfg.default_text_model.or(base.default_text_model),
        reasoning_effort: override_cfg.reasoning_effort.or(base.reasoning_effort),
        tools_file: override_cfg.tools_file.or(base.tools_file),
        skills_dir: override_cfg.skills_dir.or(base.skills_dir),
        mcp_config_path: override_cfg.mcp_config_path.or(base.mcp_config_path),
        notes_path: override_cfg.notes_path.or(base.notes_path),
        memory_path: override_cfg.memory_path.or(base.memory_path),
        allow_shell: override_cfg.allow_shell.or(base.allow_shell),
        approval_policy: override_cfg.approval_policy.or(base.approval_policy),
        sandbox_mode: override_cfg.sandbox_mode.or(base.sandbox_mode),
        managed_config_path: override_cfg
            .managed_config_path
            .or(base.managed_config_path),
        requirements_path: override_cfg.requirements_path.or(base.requirements_path),
        max_subagents: override_cfg.max_subagents.or(base.max_subagents),
        retry: override_cfg.retry.or(base.retry),
        capacity: override_cfg.capacity.or(base.capacity),
        tui: override_cfg.tui.or(base.tui),
        hooks: override_cfg.hooks.or(base.hooks),
        providers: merge_providers(base.providers, override_cfg.providers),
        features: merge_features(base.features, override_cfg.features),
        notifications: override_cfg.notifications.or(base.notifications),
        network: override_cfg.network.or(base.network),
        skills: override_cfg.skills.or(base.skills),
        snapshots: override_cfg.snapshots.or(base.snapshots),
        lsp: override_cfg.lsp.or(base.lsp),
        context: ContextConfig {
            enabled: override_cfg.context.enabled.or(base.context.enabled),
            verbatim_window_turns: override_cfg
                .context
                .verbatim_window_turns
                .or(base.context.verbatim_window_turns),
            l1_threshold: override_cfg
                .context
                .l1_threshold
                .or(base.context.l1_threshold),
            l2_threshold: override_cfg
                .context
                .l2_threshold
                .or(base.context.l2_threshold),
            l3_threshold: override_cfg
                .context
                .l3_threshold
                .or(base.context.l3_threshold),
            cycle_threshold: override_cfg
                .context
                .cycle_threshold
                .or(base.context.cycle_threshold),
            seam_model: override_cfg.context.seam_model.or(base.context.seam_model),
            per_model: override_cfg.context.per_model.or(base.context.per_model),
        },
        subagents: override_cfg.subagents.or(base.subagents),
        speech: override_cfg.speech.or(base.speech),
    }
}

fn merge_provider_config(base: ProviderConfig, override_cfg: ProviderConfig) -> ProviderConfig {
    ProviderConfig {
        api_key: override_cfg.api_key.or(base.api_key),
        base_url: override_cfg.base_url.or(base.base_url),
        model: override_cfg.model.or(base.model),
    }
}

fn merge_providers(
    base: Option<ProvidersConfig>,
    override_cfg: Option<ProvidersConfig>,
) -> Option<ProvidersConfig> {
    match (base, override_cfg) {
        (None, None) => None,
        (Some(base), None) => Some(base),
        (None, Some(override_cfg)) => Some(override_cfg),
        (Some(base), Some(override_cfg)) => Some(ProvidersConfig {
            xiaomimimo: merge_provider_config(base.xiaomimimo, override_cfg.xiaomimimo),
            nvidia_nim: merge_provider_config(base.nvidia_nim, override_cfg.nvidia_nim),
            openrouter: merge_provider_config(base.openrouter, override_cfg.openrouter),
            novita: merge_provider_config(base.novita, override_cfg.novita),
            fireworks: merge_provider_config(base.fireworks, override_cfg.fireworks),
            sglang: merge_provider_config(base.sglang, override_cfg.sglang),
        }),
    }
}

fn load_single_config_file(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let parsed: ConfigFile = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(parsed.base)
}

fn apply_managed_overrides(config: &mut Config) -> Result<()> {
    let path = config
        .managed_config_path
        .as_deref()
        .map(expand_path)
        .or_else(default_managed_config_path);
    let Some(path) = path else {
        return Ok(());
    };
    if !path.exists() {
        return Ok(());
    }
    let managed = load_single_config_file(&path)?;
    *config = merge_config(config.clone(), managed);
    Ok(())
}

fn apply_requirements(config: &mut Config) -> Result<()> {
    let path = config
        .requirements_path
        .as_deref()
        .map(expand_path)
        .or_else(default_requirements_path);
    let Some(path) = path else {
        return Ok(());
    };
    if !path.exists() {
        return Ok(());
    }
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read requirements file: {}", path.display()))?;
    let requirements: RequirementsFile = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse requirements file: {}", path.display()))?;

    if !requirements.allowed_approval_policies.is_empty()
        && let Some(policy) = config.approval_policy.as_ref()
    {
        let policy = policy.to_ascii_lowercase();
        if !requirements
            .allowed_approval_policies
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&policy))
        {
            anyhow::bail!(
                "approval_policy '{policy}' is not allowed by requirements ({})",
                requirements.allowed_approval_policies.join(", ")
            );
        }
    }
    if !requirements.allowed_sandbox_modes.is_empty()
        && let Some(mode) = config.sandbox_mode.as_ref()
    {
        let mode = mode.to_ascii_lowercase();
        if !requirements
            .allowed_sandbox_modes
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&mode))
        {
            anyhow::bail!(
                "sandbox_mode '{mode}' is not allowed by requirements ({})",
                requirements.allowed_sandbox_modes.join(", ")
            );
        }
    }

    Ok(())
}

fn merge_features(
    base: Option<FeaturesToml>,
    override_cfg: Option<FeaturesToml>,
) -> Option<FeaturesToml> {
    match (base, override_cfg) {
        (None, None) => None,
        (Some(mut base), Some(override_cfg)) => {
            for (key, value) in override_cfg.entries {
                base.entries.insert(key, value);
            }
            Some(base)
        }
        (Some(base), None) => Some(base),
        (None, Some(override_cfg)) => Some(override_cfg),
    }
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    Ok(())
}

/// Save an API key to the config file. Creates the file if it doesn't exist.
pub fn save_api_key(api_key: &str) -> Result<PathBuf> {
    fn is_api_key_assignment(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed
            .strip_prefix("api_key")
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    }

    let config_path = default_config_path()
        .context("Failed to resolve config path: home directory not found.")?;

    ensure_parent_dir(&config_path)?;

    // Don't use keychain - just write directly to config file
    // Keychain causes permission prompts on macOS for unsigned binaries
    let key_to_write = api_key.to_string();

    let content = if config_path.exists() {
        // Read existing config and update the api_key line
        let existing = fs::read_to_string(&config_path)?;
        if existing.contains("api_key") {
            // Replace existing api_key line
            let mut result = String::new();
            for line in existing.lines() {
                if is_api_key_assignment(line) {
                    let _ = writeln!(result, "api_key = \"{key_to_write}\"");
                } else {
                    result.push_str(line);
                    result.push('\n');
                }
            }
            result
        } else {
            // Prepend api_key to existing config
            format!("api_key = \"{key_to_write}\"\n{existing}")
        }
    } else {
        // Create new minimal config
        format!(
            r#"# XiaomiMiMo-TUI 配置
# 从 Xiaomi MiMo Token Plan 套餐获取 API Key，或设置 XIAOMIMIMO_API_KEY 环境变量

api_key = "{key_to_write}"

# Token Plan 套餐专属 OpenAI-compatible Base URL（默认）
# base_url = "https://token-plan-cn.xiaomimimo.com/v1"

# Default model
default_text_model = "{default_model}"

# Thinking mode (Xiaomi MiMo thinking toggle):
# "off" | "low" | "medium" | "high" | "max"
# Shift+Tab in the TUI cycles between off / high / max.
reasoning_effort = "max"

# Workspace snapshots are off by default to avoid scanning large folders
# such as Downloads before each message. Enable only inside a project folder
# if you need /restore checkpoints.
[snapshots]
enabled = false
"#,
            default_model = DEFAULT_TEXT_MODEL
        )
    };

    fs::write(&config_path, content)
        .with_context(|| format!("Failed to write config to {}", config_path.display()))?;
    log_sensitive_event(
        "credential.save",
        json!({
            "backend": "config_file",
            "config_path": config_path.display().to_string(),
        }),
    );

    Ok(config_path)
}

/// Check if an API key is configured (either in config or environment)
pub fn has_api_key(config: &Config) -> bool {
    // Check environment variable first (highest priority)
    if std::env::var("XIAOMIMIMO_API_KEY").is_ok_and(|k| !k.trim().is_empty()) {
        return true;
    }

    // Then check config file
    config
        .api_key
        .as_ref()
        .is_some_and(|k| !k.trim().is_empty() && k != API_KEYRING_SENTINEL)
}

/// Check whether the given provider has any usable API key — either via env
/// var or the corresponding `[providers.<name>]` config entry. Used by the
/// `/provider` picker to decide whether to prompt for a key inline.
#[must_use]
pub fn has_api_key_for(config: &Config, provider: ApiProvider) -> bool {
    let env_var = match provider {
        ApiProvider::XiaomiMiMo => "XIAOMIMIMO_API_KEY",
        ApiProvider::NvidiaNim => "NVIDIA_API_KEY",
        ApiProvider::Openrouter => "OPENROUTER_API_KEY",
        ApiProvider::Novita => "NOVITA_API_KEY",
        ApiProvider::Fireworks => "FIREWORKS_API_KEY",
        ApiProvider::Sglang => "SGLANG_API_KEY",
    };
    if std::env::var(env_var).is_ok_and(|k| !k.trim().is_empty()) {
        return true;
    }
    if matches!(provider, ApiProvider::NvidiaNim)
        && std::env::var("NVIDIA_NIM_API_KEY").is_ok_and(|k| !k.trim().is_empty())
    {
        return true;
    }

    // SGLang is self-hosted and typically runs without authentication.
    if matches!(provider, ApiProvider::Sglang) {
        return true;
    }

    if let Some(providers) = config.providers.as_ref() {
        let entry = match provider {
            ApiProvider::XiaomiMiMo => &providers.xiaomimimo,
            ApiProvider::NvidiaNim => &providers.nvidia_nim,
            ApiProvider::Openrouter => &providers.openrouter,
            ApiProvider::Novita => &providers.novita,
            ApiProvider::Fireworks => &providers.fireworks,
            ApiProvider::Sglang => &providers.sglang,
        };
        if entry
            .api_key
            .as_ref()
            .is_some_and(|k| !k.trim().is_empty() && k != API_KEYRING_SENTINEL)
        {
            return true;
        }
    }

    // Legacy root field is XiaomiMiMo-only.
    matches!(provider, ApiProvider::XiaomiMiMo)
        && config
            .api_key
            .as_ref()
            .is_some_and(|k| !k.trim().is_empty() && k != API_KEYRING_SENTINEL)
}

/// Save an API key to the appropriate place in `~/.xiaomimimo/config.toml` for
/// the given provider. XiaomiMiMo writes the legacy root `api_key`; other
/// providers write `[providers.<name>] api_key = "..."` (creating the table
/// if needed). Returns the config file path.
pub fn save_api_key_for(provider: ApiProvider, api_key: &str) -> Result<PathBuf> {
    if matches!(provider, ApiProvider::XiaomiMiMo) {
        return save_api_key(api_key);
    }

    let config_path = default_config_path()
        .context("Failed to resolve config path: home directory not found.")?;
    ensure_parent_dir(&config_path)?;

    let table_name = match provider {
        ApiProvider::XiaomiMiMo => unreachable!(),
        ApiProvider::NvidiaNim => "providers.nvidia_nim",
        ApiProvider::Openrouter => "providers.openrouter",
        ApiProvider::Novita => "providers.novita",
        ApiProvider::Fireworks => "providers.fireworks",
        ApiProvider::Sglang => "providers.sglang",
    };

    // Parse existing TOML (or start fresh) so we can edit the right table
    // without disturbing other sections.
    let mut doc: toml::Value = if config_path.exists() {
        let raw = fs::read_to_string(&config_path)?;
        toml::from_str(&raw)
            .with_context(|| format!("Failed to parse config at {}", config_path.display()))?
    } else {
        toml::Value::Table(toml::value::Table::new())
    };

    let table = doc
        .as_table_mut()
        .context("Config root must be a TOML table.")?;
    let providers = table
        .entry("providers".to_string())
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .context("`providers` must be a table.")?;
    let key_inside = match provider {
        ApiProvider::XiaomiMiMo => unreachable!(),
        ApiProvider::NvidiaNim => "nvidia_nim",
        ApiProvider::Openrouter => "openrouter",
        ApiProvider::Novita => "novita",
        ApiProvider::Fireworks => "fireworks",
        ApiProvider::Sglang => "sglang",
    };
    let entry = providers
        .entry(key_inside.to_string())
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .with_context(|| format!("`{table_name}` must be a table."))?;
    entry.insert(
        "api_key".to_string(),
        toml::Value::String(api_key.to_string()),
    );

    let serialized = toml::to_string_pretty(&doc).context("failed to serialize updated config")?;
    fs::write(&config_path, serialized)
        .with_context(|| format!("Failed to write config to {}", config_path.display()))?;
    log_sensitive_event(
        "credential.save",
        json!({
            "backend": "config_file",
            "provider": provider.as_str(),
            "config_path": config_path.display().to_string(),
        }),
    );

    Ok(config_path)
}

/// Clear the API key from the config file
pub fn clear_api_key() -> Result<()> {
    // Don't clear keychain - we're not using it anymore
    // Just clear from config file

    let config_path = default_config_path()
        .context("Failed to resolve config path: home directory not found.")?;

    if !config_path.exists() {
        return Ok(());
    }

    let existing = fs::read_to_string(&config_path)?;
    let mut result = String::new();

    for line in existing.lines() {
        if !line.trim_start().starts_with("api_key") {
            result.push_str(line);
            result.push('\n');
        }
    }

    fs::write(&config_path, result)
        .with_context(|| format!("Failed to write config to {}", config_path.display()))?;
    log_sensitive_event(
        "credential.clear",
        json!({
            "backend": "config_file",
            "config_path": config_path.display().to_string(),
        }),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::lock_test_env;
    use std::collections::HashMap;
    use std::env;
    use std::ffi::OsString;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct EnvGuard {
        home: Option<OsString>,
        userprofile: Option<OsString>,
        xiaomimimo_config_path: Option<OsString>,
        xiaomimimo_provider: Option<OsString>,
        xiaomimimo_api_key: Option<OsString>,
        xiaomimimo_base_url: Option<OsString>,
        xiaomimimo_model: Option<OsString>,
        xiaomimimo_default_text_model: Option<OsString>,
        nvidia_api_key: Option<OsString>,
        nvidia_nim_api_key: Option<OsString>,
        nim_base_url: Option<OsString>,
        nvidia_base_url: Option<OsString>,
        nvidia_nim_base_url: Option<OsString>,
        nvidia_nim_model: Option<OsString>,
        openrouter_api_key: Option<OsString>,
        openrouter_base_url: Option<OsString>,
        novita_api_key: Option<OsString>,
        novita_base_url: Option<OsString>,
        fireworks_api_key: Option<OsString>,
        fireworks_base_url: Option<OsString>,
        sglang_api_key: Option<OsString>,
        sglang_base_url: Option<OsString>,
        sglang_model: Option<OsString>,
    }

    impl EnvGuard {
        fn new(home: &Path) -> Self {
            let home_str = OsString::from(home.as_os_str());
            let config_path = home.join(".xiaomimimo").join("config.toml");
            let config_str = OsString::from(config_path.as_os_str());
            let home_prev = env::var_os("HOME");
            let userprofile_prev = env::var_os("USERPROFILE");
            let xiaomimimo_config_prev = env::var_os("XIAOMIMIMO_CONFIG_PATH");
            let xiaomimimo_provider_prev = env::var_os("XIAOMIMIMO_PROVIDER");
            let api_key_prev = env::var_os("XIAOMIMIMO_API_KEY");
            let base_url_prev = env::var_os("XIAOMIMIMO_BASE_URL");
            let model_prev = env::var_os("XIAOMIMIMO_MODEL");
            let default_text_model_prev = env::var_os("XIAOMIMIMO_DEFAULT_TEXT_MODEL");
            let nvidia_api_key_prev = env::var_os("NVIDIA_API_KEY");
            let nvidia_nim_api_key_prev = env::var_os("NVIDIA_NIM_API_KEY");
            let nim_base_url_prev = env::var_os("NIM_BASE_URL");
            let nvidia_base_url_prev = env::var_os("NVIDIA_BASE_URL");
            let nvidia_nim_base_url_prev = env::var_os("NVIDIA_NIM_BASE_URL");
            let nvidia_nim_model_prev = env::var_os("NVIDIA_NIM_MODEL");
            let openrouter_api_key_prev = env::var_os("OPENROUTER_API_KEY");
            let openrouter_base_url_prev = env::var_os("OPENROUTER_BASE_URL");
            let novita_api_key_prev = env::var_os("NOVITA_API_KEY");
            let novita_base_url_prev = env::var_os("NOVITA_BASE_URL");
            let fireworks_api_key_prev = env::var_os("FIREWORKS_API_KEY");
            let fireworks_base_url_prev = env::var_os("FIREWORKS_BASE_URL");
            let sglang_api_key_prev = env::var_os("SGLANG_API_KEY");
            let sglang_base_url_prev = env::var_os("SGLANG_BASE_URL");
            let sglang_model_prev = env::var_os("SGLANG_MODEL");
            // Safety: test-only environment mutation guarded by a global mutex.
            unsafe {
                env::set_var("HOME", &home_str);
                env::set_var("USERPROFILE", &home_str);
                env::set_var("XIAOMIMIMO_CONFIG_PATH", &config_str);
                env::remove_var("XIAOMIMIMO_PROVIDER");
                env::remove_var("XIAOMIMIMO_API_KEY");
                env::remove_var("XIAOMIMIMO_BASE_URL");
                env::remove_var("XIAOMIMIMO_MODEL");
                env::remove_var("XIAOMIMIMO_DEFAULT_TEXT_MODEL");
                env::remove_var("NVIDIA_API_KEY");
                env::remove_var("NVIDIA_NIM_API_KEY");
                env::remove_var("NIM_BASE_URL");
                env::remove_var("NVIDIA_BASE_URL");
                env::remove_var("NVIDIA_NIM_BASE_URL");
                env::remove_var("NVIDIA_NIM_MODEL");
                env::remove_var("OPENROUTER_API_KEY");
                env::remove_var("OPENROUTER_BASE_URL");
                env::remove_var("NOVITA_API_KEY");
                env::remove_var("NOVITA_BASE_URL");
                env::remove_var("FIREWORKS_API_KEY");
                env::remove_var("FIREWORKS_BASE_URL");
                env::remove_var("SGLANG_API_KEY");
                env::remove_var("SGLANG_BASE_URL");
                env::remove_var("SGLANG_MODEL");
            }
            Self {
                home: home_prev,
                userprofile: userprofile_prev,
                xiaomimimo_config_path: xiaomimimo_config_prev,
                xiaomimimo_provider: xiaomimimo_provider_prev,
                xiaomimimo_api_key: api_key_prev,
                xiaomimimo_base_url: base_url_prev,
                xiaomimimo_model: model_prev,
                xiaomimimo_default_text_model: default_text_model_prev,
                nvidia_api_key: nvidia_api_key_prev,
                nvidia_nim_api_key: nvidia_nim_api_key_prev,
                nim_base_url: nim_base_url_prev,
                nvidia_base_url: nvidia_base_url_prev,
                nvidia_nim_base_url: nvidia_nim_base_url_prev,
                nvidia_nim_model: nvidia_nim_model_prev,
                openrouter_api_key: openrouter_api_key_prev,
                openrouter_base_url: openrouter_base_url_prev,
                novita_api_key: novita_api_key_prev,
                novita_base_url: novita_base_url_prev,
                fireworks_api_key: fireworks_api_key_prev,
                fireworks_base_url: fireworks_base_url_prev,
                sglang_api_key: sglang_api_key_prev,
                sglang_base_url: sglang_base_url_prev,
                sglang_model: sglang_model_prev,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // Safety: test-only environment mutation guarded by a global mutex.
            unsafe {
                Self::restore_var("HOME", self.home.take());
                Self::restore_var("USERPROFILE", self.userprofile.take());
                Self::restore_var("XIAOMIMIMO_CONFIG_PATH", self.xiaomimimo_config_path.take());
                Self::restore_var("XIAOMIMIMO_PROVIDER", self.xiaomimimo_provider.take());
                Self::restore_var("XIAOMIMIMO_API_KEY", self.xiaomimimo_api_key.take());
                Self::restore_var("XIAOMIMIMO_BASE_URL", self.xiaomimimo_base_url.take());
                Self::restore_var("XIAOMIMIMO_MODEL", self.xiaomimimo_model.take());
                Self::restore_var(
                    "XIAOMIMIMO_DEFAULT_TEXT_MODEL",
                    self.xiaomimimo_default_text_model.take(),
                );
                Self::restore_var("NVIDIA_API_KEY", self.nvidia_api_key.take());
                Self::restore_var("NVIDIA_NIM_API_KEY", self.nvidia_nim_api_key.take());
                Self::restore_var("NIM_BASE_URL", self.nim_base_url.take());
                Self::restore_var("NVIDIA_BASE_URL", self.nvidia_base_url.take());
                Self::restore_var("NVIDIA_NIM_BASE_URL", self.nvidia_nim_base_url.take());
                Self::restore_var("NVIDIA_NIM_MODEL", self.nvidia_nim_model.take());
                Self::restore_var("OPENROUTER_API_KEY", self.openrouter_api_key.take());
                Self::restore_var("OPENROUTER_BASE_URL", self.openrouter_base_url.take());
                Self::restore_var("NOVITA_API_KEY", self.novita_api_key.take());
                Self::restore_var("NOVITA_BASE_URL", self.novita_base_url.take());
                Self::restore_var("FIREWORKS_API_KEY", self.fireworks_api_key.take());
                Self::restore_var("FIREWORKS_BASE_URL", self.fireworks_base_url.take());
                Self::restore_var("SGLANG_API_KEY", self.sglang_api_key.take());
                Self::restore_var("SGLANG_BASE_URL", self.sglang_base_url.take());
                Self::restore_var("SGLANG_MODEL", self.sglang_model.take());
            }
        }
    }

    impl EnvGuard {
        /// Restore an env var to its prior value (or remove it if it was unset).
        ///
        /// # Safety
        /// Must only be called from test code guarded by a global mutex.
        unsafe fn restore_var(key: &str, prev: Option<OsString>) {
            if let Some(value) = prev {
                unsafe { env::set_var(key, value) };
            } else {
                unsafe { env::remove_var(key) };
            }
        }
    }

    #[test]
    fn save_api_key_writes_config() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let path = save_api_key("test-key")?;
        let expected = temp_root.join(".xiaomimimo").join("config.toml");
        assert_eq!(path, expected);

        let contents = fs::read_to_string(&path)?;
        assert!(contents.contains("api_key = \""));
        Ok(())
    }

    #[test]
    fn test_tilde_expansion_in_paths() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-tilde-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config = Config {
            skills_dir: Some("~/.xiaomimimo/skills".to_string()),
            ..Default::default()
        };
        let expected_skills = temp_root.join(".xiaomimimo").join("skills");
        let actual_skills = config.skills_dir();
        assert_eq!(
            actual_skills.components().collect::<Vec<_>>(),
            expected_skills.components().collect::<Vec<_>>()
        );

        Ok(())
    }

    #[test]
    fn test_load_uses_tilde_expanded_xiaomimimo_config_path() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-load-tilde-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config_path = temp_root.join(".custom-xiaomimimo").join("config.toml");
        ensure_parent_dir(&config_path)?;
        fs::write(&config_path, "api_key = \"test-key\"\n")?;

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_CONFIG_PATH", "~/.custom-xiaomimimo/config.toml");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_key.as_deref(), Some("test-key"));
        Ok(())
    }

    #[test]
    fn test_load_falls_back_to_home_config_when_env_path_missing() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-load-fallback-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let home_config = temp_root.join(".xiaomimimo").join("config.toml");
        ensure_parent_dir(&home_config)?;
        fs::write(&home_config, "api_key = \"home-key\"\n")?;

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var(
                "XIAOMIMIMO_CONFIG_PATH",
                temp_root.join("missing-config.toml").as_os_str(),
            );
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_key.as_deref(), Some("home-key"));
        Ok(())
    }

    #[test]
    fn test_nonexistent_profile_error() {
        let mut profiles = HashMap::new();
        profiles.insert("work".to_string(), Config::default());
        let config = ConfigFile {
            base: Config::default(),
            profiles: Some(profiles),
        };

        let err = apply_profile(config, Some("nonexistent")).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Profile 'nonexistent' not found"));
        assert!(message.contains("Available profiles"));
        assert!(message.contains("work"));
    }

    #[test]
    fn test_profile_with_no_profiles_section() {
        let config = ConfigFile {
            base: Config::default(),
            profiles: None,
        };

        let err = apply_profile(config, Some("missing")).unwrap_err();
        assert!(err.to_string().contains("Available profiles: none"));
    }

    #[test]
    fn test_save_api_key_doesnt_match_similar_keys() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-api-key-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config_path = temp_root.join(".xiaomimimo").join("config.toml");
        ensure_parent_dir(&config_path)?;
        fs::write(
            &config_path,
            "api_key_backup = \"old\"\napi_key = \"current\"\n",
        )?;

        let path = save_api_key("new-key")?;
        assert_eq!(path, config_path);

        let contents = fs::read_to_string(&config_path)?;
        assert!(contents.contains("api_key_backup = \"old\""));
        assert!(contents.contains("api_key = \""));
        Ok(())
    }

    #[test]
    fn test_empty_api_key_rejected() {
        let config = Config {
            api_key: Some("   ".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_missing_api_key_allowed() -> Result<()> {
        let config = Config::default();
        config.validate()?;
        Ok(())
    }

    #[test]
    fn apply_env_overrides_ignores_empty_api_key() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-empty-key-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Simulate a fresh user who copied .env.example to .env without
        // filling in XIAOMIMIMO_API_KEY: dotenv loads it as the empty string.
        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_API_KEY", "");
        }

        let mut config = Config {
            api_key: Some("from-config-file".to_string()),
            ..Default::default()
        };
        apply_env_overrides(&mut config);

        assert_eq!(config.api_key.as_deref(), Some("from-config-file"));
        config.validate()?;
        Ok(())
    }

    #[test]
    fn normalize_model_name_handles_aliases_and_future_ids() {
        assert_eq!(
            normalize_model_name("xiaomimimo-v3.2").as_deref(),
            Some("mimo-v2-flash")
        );
        assert_eq!(
            normalize_model_name("xiaomimimo-r1").as_deref(),
            Some("mimo-v2-flash")
        );
        assert_eq!(
            normalize_model_name("XiaomiMiMo-V4").as_deref(),
            Some("mimo-v2.5-pro")
        );
        assert_eq!(
            normalize_model_name("mimo-v2.5-pro").as_deref(),
            Some("mimo-v2.5-pro")
        );
        assert_eq!(
            normalize_model_name("mimo-v2-flash").as_deref(),
            Some("mimo-v2-flash")
        );
        assert_eq!(
            normalize_model_name("mimo-v2-tts").as_deref(),
            Some("mimo-v2-tts")
        );
        assert_eq!(
            normalize_model_name("mimo-v2.5-tts-voiceclone").as_deref(),
            Some("mimo-v2.5-tts-voiceclone")
        );
        assert!(COMMON_XIAOMIMIMO_MODELS.contains(&"mimo-v2-tts"));
    }

    #[test]
    fn normalize_model_name_rejects_invalid_or_non_xiaomimimo_ids() {
        assert!(normalize_model_name("gpt-4o").is_none());
        assert!(normalize_model_name("xiaomimimo v4").is_none());
        assert!(normalize_model_name("").is_none());
    }

    #[test]
    fn provider_specific_custom_model_is_preserved() {
        let mut providers = ProvidersConfig::default();
        providers.openrouter.model = Some("openai/gpt-4.1-mini".to_string());
        let config = Config {
            provider: Some("openrouter".to_string()),
            providers: Some(providers),
            ..Default::default()
        };

        assert_eq!(config.default_model(), "openai/gpt-4.1-mini");
    }

    #[test]
    fn default_context_seams_are_opt_in() {
        let config = Config::default();
        assert!(!config.context.enabled.unwrap_or(false));
        assert_eq!(config.context.l1_threshold.unwrap_or(192_000), 192_000);
        assert_eq!(config.context.cycle_threshold.unwrap_or(768_000), 768_000);
        assert_eq!(
            config
                .context
                .seam_model
                .as_deref()
                .unwrap_or("mimo-v2-flash"),
            "mimo-v2-flash"
        );
    }

    #[test]
    fn profile_without_context_does_not_disable_base_context() {
        let mut profiles = HashMap::new();
        profiles.insert("work".to_string(), Config::default());
        let config = ConfigFile {
            base: Config {
                context: ContextConfig {
                    enabled: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
            profiles: Some(profiles),
        };

        let merged = apply_profile(config, Some("work")).expect("profile");
        assert_eq!(merged.context.enabled, Some(true));
    }

    #[test]
    fn validate_accepts_future_xiaomimimo_model_id() -> Result<()> {
        let config = Config {
            default_text_model: Some("mimo-v2.5-pro".to_string()),
            ..Default::default()
        };
        config.validate()?;
        Ok(())
    }

    #[test]
    fn xiaomimimo_model_env_overrides_default_text_model() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-model-env-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_MODEL", "xiaomimimo-chat");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.default_text_model.as_deref(), Some("mimo-v2-flash"));
        Ok(())
    }

    #[test]
    fn nvidia_nim_provider_uses_nim_defaults() -> Result<()> {
        let config = Config {
            provider: Some("nvidia-nim".to_string()),
            ..Default::default()
        };

        config.validate()?;
        assert_eq!(config.api_provider(), ApiProvider::NvidiaNim);
        assert_eq!(config.default_model(), DEFAULT_NVIDIA_NIM_MODEL);
        assert_eq!(config.xiaomimimo_base_url(), DEFAULT_NVIDIA_NIM_BASE_URL);
        Ok(())
    }

    #[test]
    fn nvidia_nim_provider_normalizes_xiaomimimo_pro_alias() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-nim-model-alias-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config_path = temp_root.join(".xiaomimimo").join("config.toml");
        ensure_parent_dir(&config_path)?;
        fs::write(
            &config_path,
            "provider = \"nvidia-nim\"\ndefault_text_model = \"mimo-v2.5-pro\"\napi_key = \"nim-key\"\n",
        )?;

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::NvidiaNim);
        assert_eq!(
            config.default_text_model.as_deref(),
            Some(DEFAULT_NVIDIA_NIM_MODEL)
        );
        Ok(())
    }

    #[test]
    fn nvidia_nim_provider_normalizes_xiaomimimo_flash_alias() -> Result<()> {
        let config = Config {
            provider: Some("nvidia-nim".to_string()),
            default_text_model: Some("mimo-v2-flash".to_string()),
            ..Default::default()
        };

        config.validate()?;
        assert_eq!(config.default_model(), DEFAULT_NVIDIA_NIM_FLASH_MODEL);
        Ok(())
    }

    #[test]
    fn nvidia_nim_env_overrides_provider_and_credentials() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-nim-env-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_PROVIDER", "nvidia-nim");
            env::set_var("NVIDIA_API_KEY", "nim-env-key");
            env::set_var("NVIDIA_NIM_MODEL", "mimo-v2.5-pro");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::NvidiaNim);
        assert_eq!(config.xiaomimimo_api_key()?, "nim-env-key");
        assert_eq!(config.default_model(), DEFAULT_NVIDIA_NIM_MODEL);
        Ok(())
    }

    #[test]
    fn nvidia_nim_env_accepts_short_nim_base_url_alias() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-nim-base-url-alias-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_PROVIDER", "nvidia-nim");
            env::set_var("NIM_BASE_URL", "https://short-nim.example/v1");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::NvidiaNim);
        assert_eq!(config.xiaomimimo_base_url(), "https://short-nim.example/v1");
        Ok(())
    }

    #[test]
    fn nvidia_nim_env_accepts_facade_base_url_forwarding() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-nim-forwarded-base-url-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_PROVIDER", "nvidia-nim");
            env::set_var("XIAOMIMIMO_BASE_URL", "https://forwarded-nim.example/v1");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::NvidiaNim);
        assert_eq!(
            config.xiaomimimo_base_url(),
            "https://forwarded-nim.example/v1"
        );
        Ok(())
    }

    #[test]
    fn openrouter_provider_uses_canonical_defaults() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-or-defaults-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config = Config {
            provider: Some("openrouter".to_string()),
            ..Default::default()
        };
        config.validate()?;
        assert_eq!(config.api_provider(), ApiProvider::Openrouter);
        assert_eq!(config.default_model(), DEFAULT_OPENROUTER_MODEL);
        assert_eq!(config.xiaomimimo_base_url(), DEFAULT_OPENROUTER_BASE_URL);
        Ok(())
    }

    #[test]
    fn novita_provider_uses_canonical_defaults() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-novita-defaults-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config = Config {
            provider: Some("novita".to_string()),
            ..Default::default()
        };
        config.validate()?;
        assert_eq!(config.api_provider(), ApiProvider::Novita);
        assert_eq!(config.default_model(), DEFAULT_NOVITA_MODEL);
        assert_eq!(config.xiaomimimo_base_url(), DEFAULT_NOVITA_BASE_URL);
        Ok(())
    }

    #[test]
    fn fireworks_provider_uses_canonical_defaults() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-fireworks-defaults-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config = Config {
            provider: Some("fireworks".to_string()),
            ..Default::default()
        };
        config.validate()?;
        assert_eq!(config.api_provider(), ApiProvider::Fireworks);
        assert_eq!(config.default_model(), DEFAULT_FIREWORKS_MODEL);
        assert_eq!(config.xiaomimimo_base_url(), DEFAULT_FIREWORKS_BASE_URL);
        Ok(())
    }

    #[test]
    fn sglang_provider_works_without_api_key() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-sglang-defaults-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config = Config {
            provider: Some("sglang".to_string()),
            ..Default::default()
        };
        config.validate()?;
        assert_eq!(config.api_provider(), ApiProvider::Sglang);
        assert_eq!(config.default_model(), DEFAULT_SGLANG_MODEL);
        assert_eq!(config.xiaomimimo_base_url(), DEFAULT_SGLANG_BASE_URL);
        assert_eq!(config.xiaomimimo_api_key()?, "");
        assert!(has_api_key_for(&config, ApiProvider::Sglang));
        Ok(())
    }

    #[test]
    fn openrouter_env_api_key_resolves_via_xiaomimimo_api_key() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-or-env-key-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_PROVIDER", "openrouter");
            env::set_var("OPENROUTER_API_KEY", "or-env-key");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::Openrouter);
        assert_eq!(config.xiaomimimo_api_key()?, "or-env-key");
        Ok(())
    }

    #[test]
    fn novita_env_api_key_resolves_via_xiaomimimo_api_key() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-novita-env-key-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_PROVIDER", "novita");
            env::set_var("NOVITA_API_KEY", "novita-env-key");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::Novita);
        assert_eq!(config.xiaomimimo_api_key()?, "novita-env-key");
        Ok(())
    }

    #[test]
    fn openrouter_base_url_env_overrides_default() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-or-base-url-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_PROVIDER", "openrouter");
            env::set_var("OPENROUTER_BASE_URL", "https://or-mirror.example/v1");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::Openrouter);
        assert_eq!(config.xiaomimimo_base_url(), "https://or-mirror.example/v1");
        Ok(())
    }

    #[test]
    fn facade_base_url_env_scopes_to_active_openrouter_provider() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-or-facade-base-url-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("XIAOMIMIMO_PROVIDER", "openrouter");
            env::set_var("XIAOMIMIMO_BASE_URL", "https://facade-or.example/v1");
        }

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::Openrouter);
        assert_eq!(config.base_url.as_deref(), None);
        assert_eq!(config.xiaomimimo_base_url(), "https://facade-or.example/v1");
        Ok(())
    }

    #[test]
    fn openrouter_reads_provider_table_from_config_file() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-or-table-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config_path = temp_root.join(".xiaomimimo").join("config.toml");
        ensure_parent_dir(&config_path)?;
        fs::write(
            &config_path,
            r#"provider = "openrouter"

[providers.openrouter]
api_key = "or-table-key"
base_url = "https://or-table.example/v1"
"#,
        )?;

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::Openrouter);
        assert_eq!(config.xiaomimimo_api_key()?, "or-table-key");
        assert_eq!(config.xiaomimimo_base_url(), "https://or-table.example/v1");
        Ok(())
    }

    #[test]
    fn novita_reads_provider_table_from_config_file() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-novita-table-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config_path = temp_root.join(".xiaomimimo").join("config.toml");
        ensure_parent_dir(&config_path)?;
        fs::write(
            &config_path,
            r#"provider = "novita"

[providers.novita]
api_key = "novita-table-key"
"#,
        )?;

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::Novita);
        assert_eq!(config.xiaomimimo_api_key()?, "novita-table-key");
        assert_eq!(config.xiaomimimo_base_url(), DEFAULT_NOVITA_BASE_URL);
        Ok(())
    }

    #[test]
    fn has_api_key_for_detects_env_and_config_per_provider() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-has-key-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let mut config = Config::default();
        assert!(!has_api_key_for(&config, ApiProvider::Openrouter));
        assert!(
            has_api_key_for(&config, ApiProvider::Sglang),
            "SGLang is self-hosted and does not require a key by default"
        );

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::set_var("OPENROUTER_API_KEY", "or-env");
        }
        assert!(has_api_key_for(&config, ApiProvider::Openrouter));
        assert!(!has_api_key_for(&config, ApiProvider::Novita));

        // Safety: test-only environment mutation guarded by a global mutex.
        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
        }
        let mut providers = ProvidersConfig::default();
        providers.novita.api_key = Some("file-novita".to_string());
        config.providers = Some(providers);
        assert!(has_api_key_for(&config, ApiProvider::Novita));
        assert!(!has_api_key_for(&config, ApiProvider::Openrouter));
        Ok(())
    }

    #[test]
    fn save_api_key_for_openrouter_writes_provider_table() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-save-key-or-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let path = save_api_key_for(ApiProvider::Openrouter, "or-saved-key")?;
        let contents = fs::read_to_string(&path)?;
        let parsed: toml::Value = toml::from_str(&contents)?;
        assert_eq!(
            parsed
                .get("providers")
                .and_then(|p| p.get("openrouter"))
                .and_then(|t| t.get("api_key"))
                .and_then(toml::Value::as_str),
            Some("or-saved-key")
        );
        // Re-saving must not duplicate or wipe sibling tables.
        save_api_key_for(ApiProvider::Novita, "novita-saved-key")?;
        let contents = fs::read_to_string(&path)?;
        let parsed: toml::Value = toml::from_str(&contents)?;
        assert_eq!(
            parsed
                .get("providers")
                .and_then(|p| p.get("openrouter"))
                .and_then(|t| t.get("api_key"))
                .and_then(toml::Value::as_str),
            Some("or-saved-key")
        );
        assert_eq!(
            parsed
                .get("providers")
                .and_then(|p| p.get("novita"))
                .and_then(|t| t.get("api_key"))
                .and_then(toml::Value::as_str),
            Some("novita-saved-key")
        );
        save_api_key_for(ApiProvider::Fireworks, "fireworks-saved-key")?;
        save_api_key_for(ApiProvider::Sglang, "sglang-saved-key")?;
        let contents = fs::read_to_string(&path)?;
        let parsed: toml::Value = toml::from_str(&contents)?;
        assert_eq!(
            parsed
                .get("providers")
                .and_then(|p| p.get("fireworks"))
                .and_then(|t| t.get("api_key"))
                .and_then(toml::Value::as_str),
            Some("fireworks-saved-key")
        );
        assert_eq!(
            parsed
                .get("providers")
                .and_then(|p| p.get("sglang"))
                .and_then(|t| t.get("api_key"))
                .and_then(toml::Value::as_str),
            Some("sglang-saved-key")
        );
        Ok(())
    }

    #[test]
    fn nvidia_nim_reads_facade_provider_table() -> Result<()> {
        let _lock = lock_test_env();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!(
            "xiaomimimo-tui-nim-provider-table-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&temp_root)?;
        let _guard = EnvGuard::new(&temp_root);

        let config_path = temp_root.join(".xiaomimimo").join("config.toml");
        ensure_parent_dir(&config_path)?;
        fs::write(
            &config_path,
            r#"provider = "nvidia-nim"
default_text_model = "mimo-v2-flash"

[providers.nvidia_nim]
api_key = "nim-table-key"
base_url = "https://nim-table.example/v1"
model = "mimo-v2.5-pro"
"#,
        )?;

        let config = Config::load(None, None)?;
        assert_eq!(config.api_provider(), ApiProvider::NvidiaNim);
        assert_eq!(config.xiaomimimo_api_key()?, "nim-table-key");
        assert_eq!(config.xiaomimimo_base_url(), "https://nim-table.example/v1");
        assert_eq!(config.default_model(), DEFAULT_NVIDIA_NIM_MODEL);
        Ok(())
    }

    // ========================================================================
    // Provider Capability Matrix tests
    // ========================================================================

    #[test]
    fn deprecation_for_model_returns_notice_for_legacy_aliases() {
        let cases = &[
            ("xiaomimimo-chat", "mimo-v2-flash"),
            ("xiaomimimo-reasoner", "mimo-v2-flash"),
            ("xiaomimimo-r1", "mimo-v2-flash"),
            ("xiaomimimo-v3", "mimo-v2-flash"),
            ("xiaomimimo-v3.2", "mimo-v2-flash"),
            ("xiaomimimo-v4", "mimo-v2.5-pro"),
            ("xiaomimimo-v4-pro", "mimo-v2.5-pro"),
            ("xiaomimimo-v4-flash", "mimo-v2-flash"),
        ];
        for (alias, replacement) in cases {
            let dep = deprecation_for_model(alias);
            assert!(dep.is_some(), "expected deprecation for '{alias}'");
            let dep = dep.unwrap();
            assert_eq!(dep.alias, *alias);
            assert_eq!(dep.replacement, *replacement);
            assert!(dep.notice.contains("Deprecated"));
            assert!(dep.notice.contains(replacement));
        }
    }

    #[test]
    fn deprecation_for_model_returns_none_for_current_models() {
        assert!(deprecation_for_model("mimo-v2.5-pro").is_none());
        assert!(deprecation_for_model("mimo-v2-flash").is_none());
        assert!(deprecation_for_model("mimo-v2.5-pro").is_none());
    }

    #[test]
    fn deprecation_for_model_is_case_insensitive() {
        let dep = deprecation_for_model("XiaomiMiMo-Chat").unwrap();
        assert_eq!(dep.alias, "xiaomimimo-chat");
        let dep = deprecation_for_model("XIAOMIMIMO-REASONER").unwrap();
        assert_eq!(dep.alias, "xiaomimimo-reasoner");
    }

    #[test]
    fn provider_capability_xiaomimimo_pro_has_1m_window_and_thinking() {
        let cap = provider_capability(ApiProvider::XiaomiMiMo, "mimo-v2.5-pro");
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_1M_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 131_072);
        assert!(cap.thinking_supported);
        assert!(cap.cache_telemetry_supported);
        assert_eq!(
            cap.request_payload_mode,
            RequestPayloadMode::ChatCompletions
        );
        assert!(cap.deprecation.is_none());
    }

    #[test]
    fn provider_capability_xiaomimimo_flash_has_256k_window_and_thinking() {
        let cap = provider_capability(ApiProvider::XiaomiMiMo, "mimo-v2-flash");
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_256K_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 65_536);
        assert!(cap.thinking_supported);
        assert!(cap.cache_telemetry_supported);
    }

    #[test]
    fn provider_capability_nvidia_nim_pro_maps_correctly() {
        let cap = provider_capability(ApiProvider::NvidiaNim, DEFAULT_NVIDIA_NIM_MODEL);
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_1M_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 131_072);
        assert!(cap.thinking_supported);
        assert!(!cap.cache_telemetry_supported);
        assert_eq!(
            cap.request_payload_mode,
            RequestPayloadMode::ChatCompletions
        );
    }

    #[test]
    fn provider_capability_nvidia_nim_flash_maps_correctly() {
        let cap = provider_capability(ApiProvider::NvidiaNim, DEFAULT_NVIDIA_NIM_FLASH_MODEL);
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_256K_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 65_536);
        assert!(cap.thinking_supported);
        assert!(!cap.cache_telemetry_supported);
    }

    #[test]
    fn provider_capability_openrouter_pro_has_thinking_no_cache() {
        let cap = provider_capability(ApiProvider::Openrouter, DEFAULT_OPENROUTER_MODEL);
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_1M_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 131_072);
        assert!(cap.thinking_supported);
        // OpenRouter does not return XiaomiMiMo prompt-cache telemetry.
        assert!(!cap.cache_telemetry_supported);
        assert_eq!(
            cap.request_payload_mode,
            RequestPayloadMode::ChatCompletions
        );
    }

    #[test]
    fn provider_capability_novita_pro_has_thinking_no_cache() {
        let cap = provider_capability(ApiProvider::Novita, DEFAULT_NOVITA_MODEL);
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_1M_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 131_072);
        assert!(cap.thinking_supported);
        assert!(!cap.cache_telemetry_supported);
    }

    #[test]
    fn provider_capability_fireworks_pro_has_thinking_no_cache() {
        let cap = provider_capability(ApiProvider::Fireworks, DEFAULT_FIREWORKS_MODEL);
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_1M_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 131_072);
        assert!(cap.thinking_supported);
        assert!(!cap.cache_telemetry_supported);
    }

    #[test]
    fn provider_capability_sglang_pro_has_thinking_no_cache() {
        let cap = provider_capability(ApiProvider::Sglang, DEFAULT_SGLANG_MODEL);
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_1M_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 131_072);
        assert!(cap.thinking_supported);
        assert!(!cap.cache_telemetry_supported);
    }

    #[test]
    fn provider_capability_unknown_model_has_smaller_window() {
        let cap = provider_capability(ApiProvider::XiaomiMiMo, "xiaomimimo-coder");
        assert_eq!(
            cap.context_window,
            crate::models::DEFAULT_CONTEXT_WINDOW_TOKENS
        );
        assert_eq!(cap.max_output, 4096);
        assert!(!cap.thinking_supported);
    }

    #[test]
    fn provider_capability_legacy_alias_shows_deprecation() {
        let cap = provider_capability(ApiProvider::XiaomiMiMo, "xiaomimimo-chat");
        assert!(cap.deprecation.is_some());
        let dep = cap.deprecation.unwrap();
        assert_eq!(dep.alias, "xiaomimimo-chat");
        assert_eq!(dep.replacement, "mimo-v2-flash");
        assert!(dep.notice.contains("Deprecated"));
        // Even though deprecated, it still resolves as the official flash model.
        assert_eq!(
            cap.context_window,
            crate::models::XIAOMIMIMO_256K_CONTEXT_WINDOW_TOKENS
        );
    }

    #[test]
    fn provider_capability_roundtrip_serialization() {
        let cap = provider_capability(ApiProvider::XiaomiMiMo, "mimo-v2.5-pro");
        let json = serde_json::to_value(&cap).unwrap();
        let deserialized: ProviderCapability = serde_json::from_value(json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn deprecation_serialization_roundtrip() {
        let dep = ModelDeprecation {
            alias: "xiaomimimo-chat".to_string(),
            replacement: "mimo-v2-flash".to_string(),
            notice: "Test notice".to_string(),
        };
        let json = serde_json::to_value(&dep).unwrap();
        let deserialized: ModelDeprecation = serde_json::from_value(json).unwrap();
        assert_eq!(dep, deserialized);
    }
}
