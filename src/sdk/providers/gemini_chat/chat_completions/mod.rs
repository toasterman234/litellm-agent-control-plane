pub mod transformation;

use crate::sdk::providers::base::ProviderRegistry;

pub use transformation::ChatCompletionsTransformation;

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/openai";

pub fn init(registry: &mut ProviderRegistry) {
    registry.register("gemini_chat", GEMINI_API_BASE, ChatCompletionsTransformation);
}
