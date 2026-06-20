pub mod transformation;

use crate::sdk::providers::base::ProviderRegistry;

pub use transformation::ChatCompletionsTransformation;

const MISTRAL_API_BASE: &str = "https://api.mistral.ai/v1";

pub fn init(registry: &mut ProviderRegistry) {
    registry.register("mistral", MISTRAL_API_BASE, ChatCompletionsTransformation);
}
