// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Commands that allow modifications to tile maps, tile sets, and brushes.

use fyrox::{
    core::{
        algebra::Vector2, color::Color, log::Log, pool::Handle, swap_hash_map_entry,
        ImmutableString, Uuid,
    },
    fxhash::FxHashMap,
    material::MaterialResource,
    scene::{
        node::Node,
        tilemap::{
            brush::{TileMapBrushPage, TileMapBrushResource},
            tileset::{
                AbstractTile, NamableValue, NamedValue, TileSetColliderLayer, TileSetPage,
                TileSetPageSource, TileSetPropertyLayer, TileSetPropertyType, TileSetPropertyValue,
                TileSetResource,
            },
            OrthoTransform, OrthoTransformation, TileCollider, TileDefinitionHandle, TileMap,
            TileSetUpdate, TilesUpdate,
        },
    },
};

use crate::{
    command::{CommandContext, CommandTrait},
    scene::commands::GameSceneContext,
};

#[derive(Debug)]
pub struct SetColliderLayerNameCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub name: ImmutableString,
}

impl SetColliderLayerNameCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(collider) = tile_set.find_collider_mut(self.uuid) else {
            return;
        };
        std::mem::swap(&mut collider.name, &mut self.name);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for SetColliderLayerNameCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Collider Layer Name".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
pub struct SetPropertyLayerNameCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub name: ImmutableString,
}

impl SetPropertyLayerNameCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        std::mem::swap(&mut property.name, &mut self.name);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for SetPropertyLayerNameCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Property Layer Name".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
pub struct SetColliderLayerColorCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub color: Color,
}

impl SetColliderLayerColorCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(collider) = tile_set.find_collider_mut(self.uuid) else {
            return;
        };
        std::mem::swap(&mut collider.color, &mut self.color);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for SetColliderLayerColorCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Collider Layer Color".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
pub struct SetPropertyValueColorCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub name_index: usize,
    pub color: Color,
}

impl SetPropertyValueColorCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        let Some(named_value) = property.named_values.get_mut(self.name_index) else {
            return;
        };
        std::mem::swap(&mut named_value.color, &mut self.color);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for SetPropertyValueColorCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Property Value Color".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
pub struct SetPropertyValueNameCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub name_index: usize,
    pub name: String,
}

impl SetPropertyValueNameCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        let Some(named_value) = property.named_values.get_mut(self.name_index) else {
            return;
        };
        std::mem::swap(&mut named_value.name, &mut self.name);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for SetPropertyValueNameCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Property Value Name".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
pub struct SetPropertyValueCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub name_index: usize,
    pub value: NamableValue,
}

impl SetPropertyValueCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        let Some(named_value) = property.named_values.get_mut(self.name_index) else {
            return;
        };
        std::mem::swap(&mut named_value.value, &mut self.value);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for SetPropertyValueCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Property Value Name".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
pub struct AddPropertyValueCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub value_type: TileSetPropertyType,
    pub index: usize,
}

impl CommandTrait for AddPropertyValueCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Property Value Name".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        let value = match self.value_type {
            TileSetPropertyType::I32 => NamableValue::I32(0),
            TileSetPropertyType::F32 => NamableValue::F32(0.0),
            TileSetPropertyType::String => NamableValue::I32(0),
            TileSetPropertyType::NineSlice => NamableValue::I8(0),
        };
        property.named_values.insert(
            self.index,
            NamedValue {
                value,
                ..NamedValue::default()
            },
        );
        tile_set.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        property.named_values.remove(self.index);
        tile_set.change_flag.set();
    }
}

#[derive(Debug)]
pub struct RemovePropertyValueCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub value: Option<NamedValue>,
    pub index: usize,
}

impl CommandTrait for RemovePropertyValueCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Remove Property Value Name".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        self.value = Some(property.named_values.remove(self.index));
        tile_set.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        property
            .named_values
            .insert(self.index, self.value.take().unwrap());
        tile_set.change_flag.set();
    }
}

#[derive(Debug)]
pub struct AddColliderLayerCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub index: usize,
}

impl CommandTrait for AddColliderLayerCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Collider Layer".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.colliders.insert(
            self.index,
            TileSetColliderLayer {
                uuid: self.uuid,
                ..TileSetColliderLayer::default()
            },
        );
        tile_set.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.colliders.remove(self.index);
        tile_set.change_flag.set();
    }
}

#[derive(Debug)]
pub struct AddPropertyLayerCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub index: usize,
    pub prop_type: TileSetPropertyType,
}

impl CommandTrait for AddPropertyLayerCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Property Layer".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.properties.insert(
            self.index,
            TileSetPropertyLayer {
                uuid: self.uuid,
                prop_type: self.prop_type,
                ..TileSetPropertyLayer::default()
            },
        );
        tile_set.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.properties.remove(self.index);
        tile_set.change_flag.set();
    }
}

#[derive(Debug)]
pub struct MovePropertyLayerCommand {
    pub tile_set: TileSetResource,
    pub start: usize,
    pub end: usize,
}

impl CommandTrait for MovePropertyLayerCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Property Layer".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.properties.swap(self.start, self.end);
        tile_set.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.properties.swap(self.start, self.end);
        tile_set.change_flag.set();
    }
}

#[derive(Debug)]
pub struct MovePropertyValueCommand {
    pub tile_set: TileSetResource,
    pub uuid: Uuid,
    pub start: usize,
    pub end: usize,
}

impl MovePropertyValueCommand {
    fn swap(&self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(property) = tile_set.find_property_mut(self.uuid) else {
            return;
        };
        property.named_values.swap(self.start, self.end);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for MovePropertyValueCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Property Value".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
pub struct MoveColliderLayerCommand {
    pub tile_set: TileSetResource,
    pub start: usize,
    pub end: usize,
}

impl CommandTrait for MoveColliderLayerCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Collider Layer".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.colliders.swap(self.start, self.end);
        tile_set.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.colliders.swap(self.start, self.end);
        tile_set.change_flag.set();
    }
}

#[derive(Debug)]
pub struct RemoveColliderLayerCommand {
    pub tile_set: TileSetResource,
    pub index: usize,
    pub layer: Option<TileSetColliderLayer>,
    pub values: FxHashMap<TileDefinitionHandle, TileCollider>,
}

impl CommandTrait for RemoveColliderLayerCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Remove Collider Layer".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        let layer = tile_set.colliders.remove(self.index);
        let uuid = layer.uuid;
        self.layer = Some(layer);
        tile_set.swap_all_values_for_collider(uuid, &mut self.values);
        tile_set.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        let layer = self.layer.take().unwrap();
        let uuid = layer.uuid;
        tile_set.colliders.insert(self.index, layer);
        tile_set.swap_all_values_for_collider(uuid, &mut self.values);
        tile_set.change_flag.set();
    }
}

#[derive(Debug)]
pub struct RemovePropertyLayerCommand {
    pub tile_set: TileSetResource,
    pub index: usize,
    pub layer: Option<TileSetPropertyLayer>,
    pub values: FxHashMap<TileDefinitionHandle, TileSetPropertyValue>,
}

impl CommandTrait for RemovePropertyLayerCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Remove Property Layer".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        let layer = tile_set.properties.remove(self.index);
        let uuid = layer.uuid;
        self.layer = Some(layer);
        tile_set.swap_all_values_for_property(uuid, &mut self.values);
        tile_set.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut tile_set = self.tile_set.data_ref();
        let layer = self.layer.take().unwrap();
        let uuid = layer.uuid;
        tile_set.properties.insert(self.index, layer);
        tile_set.swap_all_values_for_property(uuid, &mut self.values);
        tile_set.change_flag.set();
    }
}

#[derive(Debug)]
pub struct SetBrushPageCommand {
    pub brush: TileMapBrushResource,
    pub position: Vector2<i32>,
    pub page: Option<TileMapBrushPage>,
}

impl SetBrushPageCommand {
    fn swap(&mut self) {
        let mut brush = self.brush.data_ref();
        swap_hash_map_entry(brush.pages.entry(self.position), &mut self.page);
        brush.change_flag.set();
    }
}

impl CommandTrait for SetBrushPageCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Brush Page".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct SetTileSetPageCommand {
    pub tile_set: TileSetResource,
    pub position: Vector2<i32>,
    pub page: Option<TileSetPage>,
}

impl SetTileSetPageCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        swap_hash_map_entry(tile_set.pages.entry(self.position), &mut self.page);
        tile_set.rebuild_transform_sets();
        tile_set.rebuild_animations();
        tile_set.change_flag.set();
    }
}

impl CommandTrait for SetTileSetPageCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Tile Set Page".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct MoveTileSetPageCommand {
    pub tile_set: TileSetResource,
    pub pages: Vec<Vector2<i32>>,
    data: Vec<Option<TileSetPage>>,
    pub start_offset: Vector2<i32>,
    pub end_offset: Vector2<i32>,
}

impl MoveTileSetPageCommand {
    pub fn new(tile_set: TileSetResource, pages: Vec<Vector2<i32>>, offset: Vector2<i32>) -> Self {
        Self {
            data: vec![None; pages.len()],
            tile_set,
            pages,
            start_offset: Vector2::new(0, 0),
            end_offset: offset,
        }
    }
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        for (i, p) in self.pages.iter().enumerate() {
            swap_hash_map_entry(
                tile_set.pages.entry(*p + self.start_offset),
                &mut self.data[i],
            );
        }
        for (i, p) in self.pages.iter().enumerate() {
            swap_hash_map_entry(
                tile_set.pages.entry(*p + self.end_offset),
                &mut self.data[i],
            );
        }
        std::mem::swap(&mut self.start_offset, &mut self.end_offset);
        tile_set.rebuild_transform_sets();
        tile_set.rebuild_animations();
        tile_set.change_flag.set();
    }
}

impl CommandTrait for MoveTileSetPageCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Tile Set Page".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct MoveBrushPageCommand {
    pub brush: TileMapBrushResource,
    pub pages: Vec<Vector2<i32>>,
    data: Vec<Option<TileMapBrushPage>>,
    pub start_offset: Vector2<i32>,
    pub end_offset: Vector2<i32>,
}

impl MoveBrushPageCommand {
    pub fn new(
        brush: TileMapBrushResource,
        pages: Vec<Vector2<i32>>,
        offset: Vector2<i32>,
    ) -> Self {
        Self {
            data: vec![None; pages.len()],
            brush,
            pages,
            start_offset: Vector2::new(0, 0),
            end_offset: offset,
        }
    }
    fn swap(&mut self) {
        let mut brush = self.brush.data_ref();
        for (i, p) in self.pages.iter().enumerate() {
            swap_hash_map_entry(brush.pages.entry(*p + self.start_offset), &mut self.data[i]);
        }
        for (i, p) in self.pages.iter().enumerate() {
            swap_hash_map_entry(brush.pages.entry(*p + self.end_offset), &mut self.data[i]);
        }
        std::mem::swap(&mut self.start_offset, &mut self.end_offset);
        brush.change_flag.set();
    }
}

impl CommandTrait for MoveBrushPageCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Brush Page".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct MoveTileSetTileCommand {
    pub tile_set: TileSetResource,
    pub page: Vector2<i32>,
    pub tiles: Vec<Vector2<i32>>,
    data: Vec<Option<AbstractTile>>,
    pub start_offset: Vector2<i32>,
    pub end_offset: Vector2<i32>,
}

impl MoveTileSetTileCommand {
    pub fn new(
        tile_set: TileSetResource,
        page: Vector2<i32>,
        tiles: Vec<Vector2<i32>>,
        offset: Vector2<i32>,
    ) -> Self {
        Self {
            data: vec![None; tiles.len()],
            tile_set,
            page,
            tiles,
            start_offset: Vector2::new(0, 0),
            end_offset: offset,
        }
    }
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        for (i, p) in self.tiles.iter().enumerate() {
            self.data[i] =
                tile_set.set_abstract_tile(self.page, *p + self.start_offset, self.data[i].take());
        }
        for (i, p) in self.tiles.iter().enumerate() {
            self.data[i] =
                tile_set.set_abstract_tile(self.page, *p + self.end_offset, self.data[i].take());
        }
        std::mem::swap(&mut self.start_offset, &mut self.end_offset);
        tile_set.rebuild_transform_sets();
        tile_set.rebuild_animations();
        tile_set.change_flag.set();
    }
}

impl CommandTrait for MoveTileSetTileCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Tile in Tile Set".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct MoveBrushTileCommand {
    pub brush: TileMapBrushResource,
    pub page: Vector2<i32>,
    pub tiles: Vec<Vector2<i32>>,
    data: Vec<Option<TileDefinitionHandle>>,
    pub start_offset: Vector2<i32>,
    pub end_offset: Vector2<i32>,
}

impl MoveBrushTileCommand {
    pub fn new(
        brush: TileMapBrushResource,
        page: Vector2<i32>,
        tiles: Vec<Vector2<i32>>,
        offset: Vector2<i32>,
    ) -> Self {
        Self {
            data: vec![None; tiles.len()],
            brush,
            page,
            tiles,
            start_offset: Vector2::new(0, 0),
            end_offset: offset,
        }
    }
    fn swap(&mut self) {
        let mut brush = self.brush.data_ref();
        let Some(page) = brush.pages.get_mut(&self.page) else {
            Log::err("Move brush tile on non-existent page.");
            return;
        };
        for (i, p) in self.tiles.iter().enumerate() {
            swap_hash_map_entry(page.tiles.entry(*p + self.start_offset), &mut self.data[i]);
        }
        for (i, p) in self.tiles.iter().enumerate() {
            swap_hash_map_entry(page.tiles.entry(*p + self.end_offset), &mut self.data[i]);
        }
        std::mem::swap(&mut self.start_offset, &mut self.end_offset);
        brush.change_flag.set();
    }
}

impl CommandTrait for MoveBrushTileCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Tile in Brush".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct MoveMapTileCommand {
    pub tile_map: Handle<Node>,
    pub tiles: Vec<Vector2<i32>>,
    data: Vec<Option<TileDefinitionHandle>>,
    pub start_offset: Vector2<i32>,
    pub end_offset: Vector2<i32>,
}

impl MoveMapTileCommand {
    pub fn new(tile_map: Handle<Node>, tiles: Vec<Vector2<i32>>, offset: Vector2<i32>) -> Self {
        Self {
            data: vec![None; tiles.len()],
            tile_map,
            tiles,
            start_offset: Vector2::new(0, 0),
            end_offset: offset,
        }
    }
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let tile_map = context.scene.graph[self.tile_map]
            .cast_mut::<TileMap>()
            .expect("Cast to TileMap failed!");
        let Some(mut tiles) = tile_map.tiles().map(|r| r.data_ref()) else {
            return;
        };
        let Some(tiles) = tiles.as_loaded_mut() else {
            return;
        };
        for (i, p) in self.tiles.iter().enumerate() {
            let data = &mut self.data[i];
            *data = tiles.replace(*p + self.start_offset, *data);
        }
        for (i, p) in self.tiles.iter().enumerate() {
            let data = &mut self.data[i];
            *data = tiles.replace(*p + self.end_offset, *data);
        }
        std::mem::swap(&mut self.start_offset, &mut self.end_offset);
    }
}

impl CommandTrait for MoveMapTileCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Tile".into()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct TransformTilesCommand {
    pub tile_set: TileSetResource,
    pub page: Vector2<i32>,
    pub tiles: Vec<Vector2<i32>>,
    pub transformation: OrthoTransformation,
}

impl TransformTilesCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(source) = tile_set.pages.get_mut(&self.page).map(|p| &mut p.source) else {
            Log::err("Transform tile command on non-existent page.");
            return;
        };
        let TileSetPageSource::Freeform(map) = source else {
            Log::err("Transform tile command on non-freeform tiles.");
            return;
        };
        for p in self.tiles.iter() {
            let Some(def) = map.get_mut(p) else {
                continue;
            };
            def.material_bounds.bounds = def
                .material_bounds
                .bounds
                .clone()
                .transformed(self.transformation);
        }
        self.transformation = self.transformation.inverted();
        tile_set.change_flag.set();
    }
}

impl CommandTrait for TransformTilesCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Transform Tiles in Tile Set".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct SetBrushTilesCommand {
    pub brush: TileMapBrushResource,
    pub page: Vector2<i32>,
    pub tiles: TilesUpdate,
}

impl SetBrushTilesCommand {
    fn swap(&mut self) {
        let mut brush = self.brush.data_ref();
        if let Some(page) = brush.pages.get_mut(&self.page) {
            page.tiles.swap_tiles(&mut self.tiles);
        }
        brush.change_flag.set();
    }
}

impl CommandTrait for SetBrushTilesCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Brush Tiles".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct SetTileSetTilesCommand {
    pub tile_set: TileSetResource,
    pub tiles: TileSetUpdate,
}

impl SetTileSetTilesCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        tile_set.swap(&mut self.tiles);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for SetTileSetTilesCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Tile Set Tiles".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct SetMapTilesCommand {
    pub tile_map: Handle<Node>,
    pub tiles: TilesUpdate,
}

impl SetMapTilesCommand {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let tile_map = context.scene.graph[self.tile_map]
            .cast_mut::<TileMap>()
            .expect("Cast to TileMap failed!");
        let Some(mut tiles) = tile_map.tiles().map(|r| r.data_ref()) else {
            return;
        };
        let Some(tiles) = tiles.as_loaded_mut() else {
            return;
        };
        tiles.swap_tiles(&mut self.tiles);
    }
}

impl CommandTrait for SetMapTilesCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Draw Tiles".into()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct ModifyAnimationSpeedCommand {
    pub tile_set: TileSetResource,
    pub page: Vector2<i32>,
    pub frame_rate: f32,
}

impl ModifyAnimationSpeedCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(TileSetPageSource::Animation(anim)) =
            &mut tile_set.pages.get_mut(&self.page).map(|p| &mut p.source)
        else {
            Log::err("Modify animation speed on non-animation tile page.");
            return;
        };
        std::mem::swap(&mut self.frame_rate, &mut anim.frame_rate);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for ModifyAnimationSpeedCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Tile Animation Speed".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct ModifyPageTileSizeCommand {
    pub tile_set: TileSetResource,
    pub page: Vector2<i32>,
    pub size: Vector2<u32>,
}

impl ModifyPageTileSizeCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(TileSetPageSource::Atlas(mat)) =
            &mut tile_set.pages.get_mut(&self.page).map(|p| &mut p.source)
        else {
            Log::err("Modify tile size on non-material tile page.");
            return;
        };
        std::mem::swap(&mut self.size, &mut mat.tile_size);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for ModifyPageTileSizeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Tile Size".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct ModifyPageMaterialCommand {
    pub tile_set: TileSetResource,
    pub page: Vector2<i32>,
    pub material: MaterialResource,
}

impl ModifyPageMaterialCommand {
    fn swap(&mut self) {
        let mut tile_set = self.tile_set.data_ref();
        let Some(TileSetPageSource::Atlas(mat)) =
            &mut tile_set.pages.get_mut(&self.page).map(|p| &mut p.source)
        else {
            Log::err("Modify tile page material on non-material page.");
            return;
        };
        std::mem::swap(&mut self.material, &mut mat.material);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for ModifyPageMaterialCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Tile Page Material".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct ModifyPageIconCommand {
    pub tile_set: TileSetResource,
    pub page: Vector2<i32>,
    pub icon: TileDefinitionHandle,
    error: bool,
}

impl ModifyPageIconCommand {
    pub fn new(tile_set: TileSetResource, page: Vector2<i32>, icon: TileDefinitionHandle) -> Self {
        Self {
            tile_set,
            page,
            icon,
            error: false,
        }
    }
    fn swap(&mut self) {
        if self.error {
            return;
        }
        let mut tile_set = self.tile_set.data_ref();
        let Some(page) = &mut tile_set.pages.get_mut(&self.page) else {
            Log::err("Modify icon of non-existent tile page.");
            self.error = true;
            return;
        };
        std::mem::swap(&mut self.icon, &mut page.icon);
        tile_set.change_flag.set();
    }
}

impl CommandTrait for ModifyPageIconCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Tile Page Icon".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct ModifyBrushPageIconCommand {
    pub brush: TileMapBrushResource,
    pub page: Vector2<i32>,
    pub icon: TileDefinitionHandle,
    error: bool,
}

impl ModifyBrushPageIconCommand {
    pub fn new(
        brush: TileMapBrushResource,
        page: Vector2<i32>,
        icon: TileDefinitionHandle,
    ) -> Self {
        Self {
            brush,
            page,
            icon,
            error: false,
        }
    }
    fn swap(&mut self) {
        if self.error {
            return;
        }
        let mut brush = self.brush.data_ref();
        let Some(page) = &mut brush.pages.get_mut(&self.page) else {
            Log::err("Modify icon of non-existent tile page.");
            self.error = true;
            return;
        };
        std::mem::swap(&mut self.icon, &mut page.icon);
        brush.change_flag.set();
    }
}

impl CommandTrait for ModifyBrushPageIconCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Tile Page Icon".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct ModifyBrushTileSetCommand {
    pub brush: TileMapBrushResource,
    pub tile_set: Option<TileSetResource>,
}

impl ModifyBrushTileSetCommand {
    fn swap(&mut self) {
        let mut brush = self.brush.data_ref();
        std::mem::swap(&mut self.tile_set, &mut brush.tile_set);
        brush.change_flag.set();
    }
}

impl CommandTrait for ModifyBrushTileSetCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Choose Tile Set for Brush".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}
