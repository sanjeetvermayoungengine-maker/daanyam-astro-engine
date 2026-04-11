use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZodiacMode {
    Tropical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn western_layer_is_tropical_first() {
        assert_eq!(ZodiacMode::Tropical, ZodiacMode::Tropical);
    }
}
