use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelKind {
    SmallLocal,
    LargeLocal,
    Remote,
    Vision,
    Specialized,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRoute {
    pub kind: ModelKind,
    pub model_id: String,
    pub reason: String,
}

pub struct ModelRouter;

impl ModelRouter {
    pub fn route(
        complexity: u8,
        private: bool,
        needs_vision: bool,
        specialized: Option<&str>,
        local_available: bool,
    ) -> ModelRoute {
        if needs_vision {
            return ModelRoute {
                kind: ModelKind::Vision,
                model_id: "vision-local".into(),
                reason: "vision required".into(),
            };
        }
        if let Some(name) = specialized {
            return ModelRoute {
                kind: ModelKind::Specialized,
                model_id: name.into(),
                reason: "specialized capability".into(),
            };
        }
        if private || !local_available {
            let kind = if complexity > 7 && local_available {
                ModelKind::LargeLocal
            } else {
                ModelKind::SmallLocal
            };
            return ModelRoute {
                kind,
                model_id: if complexity > 7 {
                    "local-large".into()
                } else {
                    "local-small".into()
                },
                reason: if private {
                    "privacy prefers local".into()
                } else {
                    "local preferred".into()
                },
            };
        }
        if complexity > 8 {
            ModelRoute {
                kind: ModelKind::Remote,
                model_id: "remote-general".into(),
                reason: "high complexity".into(),
            }
        } else {
            ModelRoute {
                kind: ModelKind::SmallLocal,
                model_id: "local-small".into(),
                reason: "default local".into(),
            }
        }
    }
}
