use deps::opentelemetry;
use deps::opentelemetry::{global, propagation::Extractor, propagation::Injector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ブログにあった感じで、injector/extractor を実装してみる。
// https://peterpaul.hashnode.dev/opentelemetry-context-propagation-with-rust
// Serializable datastructure to hold the opentelemetry propagation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationContext(pub HashMap<String, String>);

impl PropagationContext {
    fn empty() -> Self {
        Self(HashMap::new())
    }

    pub fn inject(context: &opentelemetry::Context) -> Self {
        global::get_text_map_propagator(|propagator| {
            let mut propagation_context = PropagationContext::empty();
            propagator.inject_context(context, &mut propagation_context);
            propagation_context
        })
    }

    pub fn extract(&self) -> opentelemetry::Context {
        global::get_text_map_propagator(|propagator| propagator.extract(self))
    }
}

impl Injector for PropagationContext {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
}

impl Extractor for PropagationContext {
    fn get(&self, key: &str) -> Option<&str> {
        let key = key.to_owned();
        self.0.get(&key).map(|v| v.as_ref())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_ref()).collect()
    }
}
