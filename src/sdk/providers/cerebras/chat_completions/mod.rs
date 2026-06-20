pub mod transformation;

use crate::sdk::providers::base::ProviderRegistry;

pub use transformation::ChatCompletionsTransformation;

const CEREBRAS_API_BASE: &str = "https://api.cerebras.ai/v1";

pub fn init(registry: &mut ProviderRegistry) {
    registry.register("cerebras", CEREBRAS_API_BASE, ChatCompletionsTransformation);
}
