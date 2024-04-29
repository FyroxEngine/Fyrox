use crate::{
    brush::Brush,
    core::{
        algebra::Vector2,
        math::curve::{Curve, CurveKey, CurveKeyKind},
        reflect::prelude::*,
        uuid::Uuid,
        visitor::prelude::*,
    },
};
use std::cmp::Ordering;

#[derive(Default, Clone, Debug, Visit, Reflect)]
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

#[derive(Default, Clone, Visit, Reflect, Debug)]
pub struct CurveKeyViewContainer {
    id: Uuid,
    pub brush: Brush,
    keys: Vec<CurveKeyView>,
}

impl CurveKeyViewContainer {
    pub fn new(curve: &Curve, brush: Brush) -> Self {
        Self {
            keys: curve
                .keys()
                .iter()
                .map(CurveKeyView::from)
                .collect::<Vec<_>>(),
            brush,
            id: curve.id(),
        }
    }

    pub fn add(&mut self, key: CurveKeyView) {
        self.keys.push(key)
    }

    pub fn remove(&mut self, id: Uuid) -> Option<CurveKeyView> {
        if let Some(position) = self.keys.iter().position(|k| k.id == id) {
            Some(self.keys.remove(position))
        } else {
            None
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn key_ref(&self, id: Uuid) -> Option<&CurveKeyView> {
        self.keys.iter().find(|k| k.id == id)
    }

    pub fn key_mut(&mut self, id: Uuid) -> Option<&mut CurveKeyView> {
        self.keys.iter_mut().find(|k| k.id == id)
    }

    pub fn key_position(&self, id: Uuid) -> Option<usize> {
        self.keys.iter().position(|key| key.id == id)
    }

    pub fn key_index_ref(&self, index: usize) -> Option<&CurveKeyView> {
        self.keys.get(index)
    }

    pub fn key_index_mut(&mut self, index: usize) -> Option<&mut CurveKeyView> {
        self.keys.get_mut(index)
    }

    pub fn keys(&self) -> &[CurveKeyView] {
        &self.keys
    }

    pub fn keys_mut(&mut self) -> &mut [CurveKeyView] {
        &mut self.keys
    }

    pub fn sort_keys(&mut self) {
        self.keys.sort_by(|a, b| {
            if a.position.x < b.position.x {
                Ordering::Less
            } else if a.position.x > b.position.x {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
    }

    pub fn curve(&self) -> Curve {
        let mut curve = Curve::from(
            self.keys
                .iter()
                .map(|k| {
                    let mut key = CurveKey::new(k.position.x, k.position.y, k.kind.clone());
                    key.id = k.id;
                    key
                })
                .collect::<Vec<_>>(),
        );
        curve.set_id(self.id);
        curve
    }
}
