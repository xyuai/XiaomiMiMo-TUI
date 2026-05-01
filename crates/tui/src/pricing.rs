//! Cost estimation for XiaomiMiMo API usage.
//!
//! Pricing based on XiaomiMiMo's published rates (per million tokens).

use crate::models::Usage;

/// Per-million-token pricing for a model.
struct ModelPricing {
    input_cache_hit_per_million: f64,
    input_cache_miss_per_million: f64,
    output_per_million: f64,
}

/// Look up pricing for a model name.
fn pricing_for_model(model: &str) -> Option<ModelPricing> {
    let lower = model.to_lowercase();
    if !(lower.contains("mimo") || lower.contains("xiaomimimo")) {
        return None;
    }

    if lower.contains("mimo-v2.5-pro") || lower.contains("mimo-v2-pro") || lower.contains("mimo-pro") {
        Some(ModelPricing {
            input_cache_hit_per_million: 0.20,
            input_cache_miss_per_million: 1.00,
            output_per_million: 3.00,
        })
    } else if lower == "mimo-v2.5" || lower.contains("mimo-v2-omni") {
        Some(ModelPricing {
            input_cache_hit_per_million: 0.08,
            input_cache_miss_per_million: 0.40,
            output_per_million: 2.00,
        })
    } else if lower.contains("mimo-v2-flash") || lower.contains("mimo-flash") || lower.contains("xiaomimimo-chat") {
        Some(ModelPricing {
            input_cache_hit_per_million: 0.01,
            input_cache_miss_per_million: 0.10,
            output_per_million: 0.30,
        })
    } else {
        None
    }
}

/// Calculate cost for a turn given token usage and model.
#[must_use]
#[allow(dead_code)]
pub fn calculate_turn_cost(model: &str, input_tokens: u32, output_tokens: u32) -> Option<f64> {
    let pricing = pricing_for_model(model)?;
    Some(calculate_turn_cost_with_pricing(
        pricing,
        input_tokens,
        output_tokens,
    ))
}

fn calculate_turn_cost_with_pricing(
    pricing: ModelPricing,
    input_tokens: u32,
    output_tokens: u32,
) -> f64 {
    let input_cost = (input_tokens as f64 / 1_000_000.0) * pricing.input_cache_miss_per_million;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * pricing.output_per_million;
    input_cost + output_cost
}

/// Calculate cost from provider usage, honoring XiaomiMiMo context-cache fields.
#[must_use]
pub fn calculate_turn_cost_from_usage(model: &str, usage: &Usage) -> Option<f64> {
    let pricing = pricing_for_model(model)?;
    Some(calculate_turn_cost_from_usage_with_pricing(pricing, usage))
}

fn calculate_turn_cost_from_usage_with_pricing(pricing: ModelPricing, usage: &Usage) -> f64 {
    let hit_tokens = usage.prompt_cache_hit_tokens.unwrap_or(0);
    let miss_tokens = usage
        .prompt_cache_miss_tokens
        .unwrap_or_else(|| usage.input_tokens.saturating_sub(hit_tokens));
    let accounted_input = hit_tokens.saturating_add(miss_tokens);
    let uncategorized_input = usage.input_tokens.saturating_sub(accounted_input);

    let hit_cost = (hit_tokens as f64 / 1_000_000.0) * pricing.input_cache_hit_per_million;
    let miss_cost = ((miss_tokens.saturating_add(uncategorized_input)) as f64 / 1_000_000.0)
        * pricing.input_cache_miss_per_million;
    let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * pricing.output_per_million;
    hit_cost + miss_cost + output_cost
}

/// Format a USD cost for compact display.
#[must_use]
#[allow(dead_code)]
pub fn format_cost(cost: f64) -> String {
    if cost < 0.0001 {
        "<$0.0001".to_string()
    } else if cost < 0.01 {
        format!("${:.4}", cost)
    } else if cost < 1.0 {
        format!("${:.3}", cost)
    } else {
        format!("${:.2}", cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro_models_use_published_overseas_rates() {
        let pricing = pricing_for_model("mimo-v2.5-pro").unwrap();
        assert_eq!(pricing.input_cache_hit_per_million, 0.20);
        assert_eq!(pricing.input_cache_miss_per_million, 1.00);
        assert_eq!(pricing.output_per_million, 3.00);
    }

    #[test]
    fn v25_and_omni_use_mid_tier_rates() {
        let pricing = pricing_for_model("mimo-v2.5").unwrap();
        assert_eq!(pricing.input_cache_hit_per_million, 0.08);
        assert_eq!(pricing.input_cache_miss_per_million, 0.40);
        assert_eq!(pricing.output_per_million, 2.00);
    }

    #[test]
    fn flash_uses_published_flash_rates() {
        let pricing = pricing_for_model("mimo-v2-flash").unwrap();
        assert_eq!(pricing.input_cache_hit_per_million, 0.01);
        assert_eq!(pricing.input_cache_miss_per_million, 0.10);
        assert_eq!(pricing.output_per_million, 0.30);
    }
}
