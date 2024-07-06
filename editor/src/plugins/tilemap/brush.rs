use crate::fyrox::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
    },
    scene::debug::SceneDrawingContext,
};

#[derive(Default, PartialEq, Debug, Clone)]
pub struct BrushTile {
    pub definition_index: usize,
    pub local_position: Vector2<i32>,
}

impl BrushTile {
    pub fn draw_outline(
        &self,
        ctx: &mut SceneDrawingContext,
        position: Vector2<i32>,
        world_transform: &Matrix4<f32>,
        color: Color,
    ) {
        ctx.draw_rectangle(
            0.5,
            0.5,
            Matrix4::new_translation(
                &((self.local_position + position)
                    .cast::<f32>()
                    .to_homogeneous()
                    + Vector3::new(0.5, 0.5, 0.0)),
            ) * world_transform,
            color,
        );
    }
}

#[derive(Default, PartialEq, Debug, Clone)]
pub struct TileMapBrush {
    pub tiles: Vec<BrushTile>,
}

impl TileMapBrush {
    pub fn draw_outline(
        &self,
        ctx: &mut SceneDrawingContext,
        position: Vector2<i32>,
        world_transform: &Matrix4<f32>,
        color: Color,
    ) {
        for tile in self.tiles.iter() {
            tile.draw_outline(ctx, position, world_transform, color);
        }
    }
}
