use crate::core::{algebra::Vector2, curve::CurveKey, curve::CurveKeyKind, uuid::Uuid};

#[derive(Clone, Debug)]
pub struct CurveKeyView {
    pub position: Vector2<f32>,
    pub kind: CurveKeyKind,
    pub id: Uuid,
}

impl From<&CurveKey> for CurveKeyView {
    fn from(key: &CurveKey) -> Self {
        Self {
            position: Vector2::new(key.location(), key.value),
            kind: key.kind.clone(),
            id: key.id,
        }
    }
}
