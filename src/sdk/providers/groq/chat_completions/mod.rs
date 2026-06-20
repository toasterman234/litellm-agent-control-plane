pub mod transformation;

use crate::sdk::providers::base::ProviderRegistry;

pub use transformation::ChatCompletionsTransformation;

const GROQ_API_BASE: &str = "https://api.groq.com/openai/v1";

pub fn init(registry: &mut ProviderRegistry) {
    registry.register("groq", GROQ_API_BASE, ChatCompletionsTransformation);
}
