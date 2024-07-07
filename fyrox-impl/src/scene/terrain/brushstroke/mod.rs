//! The brushstroke module contains tools for modifying terrain textures.
//! It uses a triple-buffer system to separate the UI mouse movements
//! from the update of the data within the actual textures.
//! 1. The first buffer is a [std::sync::mpsc::channel] that is used to send
//! messages to control a thread that processes brush strokes.
//! These messages are [BrushThreadMessage]. Some of the messages
//! are processed as soon as they are received, but [BrushThreadMessage::Pixel]
//! messages are sent to the next buffer.
//! 2. The pixel message buffer holds a limited number of pixel messages.
//! It serves to merge redundent pixel messages to spare the thread from
//! repeating work. It is expected that brush operations will paint multiple
//! times to the same pixel in quick succession.
//! Once the new value for a pixel has been calculated, the value is stored
//! in the third buffer.
//! 3. The [StrokeData] buffer stores every change that a particular brush stroke
//! has made to the texture. Because modifying a texture is a nontrivial operation,
//! modified pixels are allowed to accumulate to some quantity before the new pixel
//! values are actually written to the textures of the terrain.
//! [StrokeChunks] is used to keep track of which pixels are waiting to be written
//! to which terrain chunks.
use super::Chunk;
use crate::asset::ResourceDataRef;
use crate::core::{
    algebra::{Matrix2, Vector2},
    log::Log,
    math::Rect,
    pool::Handle,
    reflect::prelude::*,
};
use crate::fxhash::FxHashMap;
use crate::resource::texture::{Texture, TextureResource};
use crate::scene::node::Node;
use fyrox_core::uuid_provider;
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, SendError, Sender};

pub mod brushraster;
use brushraster::*;
pub mod strokechunks;
use strokechunks::*;

/// The number of pixel messages we can accept at once before we must start processing them.
/// Often later messages will cause earlier messages to be unnecessary, so it can be more efficient
/// to let some messages accumulate rather than process each message one-at-a-time.
const MESSAGE_BUFFER_SIZE: usize = 40;
/// The number of processed pixels we can hold before we must write the pixels to the targetted textures.
/// Modifying a texture is expensive, so it is important to do it in batches of multiple pixels.
const PIXEL_BUFFER_SIZE: usize = 40;
/// The maximum number of pixels that are allowed to be involved in a single step of a brushstroke.
/// This limit is arbitrarily chosen, but there should be some limit to prevent the editor
/// from freezing as a result of an excessively large brush.
const BRUSH_PIXEL_SANITY_LIMIT: i32 = 1000000;

#[inline]
fn mask_raise(original: u8, amount: f32) -> u8 {
    (original as f32 + amount * 255.0).clamp(0.0, 255.0) as u8
}

#[inline]
fn mask_lerp(original: u8, value: f32, t: f32) -> u8 {
    let original = original as f32;
    let value = value * 255.0;
    (original * (1.0 - t) + value * t).clamp(0.0, 255.0) as u8
}

/// A message that can be sent to the terrain painting thread to control the painting.
#[derive(Debug, Clone)]
pub enum BrushThreadMessage {
    /// Set the brush that will be used for future pixels and the textures that will be modified.
    StartStroke(Brush, Handle<Node>, TerrainTextureData),
    /// No futher pixels will be sent for the current stroke.
    EndStroke,
    /// Paint the given pixel as part of the current stroke.
    Pixel(BrushPixelMessage),
}

/// A message that can be sent to indicate that the pixel at the given coordinates
/// should be painted with the given alpha.
#[derive(Debug, Clone)]
pub struct BrushPixelMessage {
    /// The coordinates of the pixel to paint.
    pub position: Vector2<i32>,
    /// The transparency of the brush, from 0.0 for transparent to 1.0 for opaque.
    pub alpha: f32,
    /// A value whose meaning depends on the brush.
    /// For flatten brushes, this is the target height.
    pub value: f32,
}

/// A queue that stores pixels that are waiting to be drawn by the brush.
pub struct PixelMessageBuffer {
    data: VecDeque<BrushPixelMessage>,
    max_size: usize,
}

impl PixelMessageBuffer {
    /// Create a new buffer with the given size.
    #[inline]
    pub fn new(max_size: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_size),
            max_size,
        }
    }
    /// True if the buffer has reached its maximum size.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.max_size == self.data.len()
    }
    /// True if there is nothing to pop from the queue.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    /// Remove the message from the front of the queue and return it, if the queue is not empty.
    #[inline]
    pub fn pop(&mut self) -> Option<BrushPixelMessage> {
        self.data.pop_front()
    }
    /// Push a message onto the back of the queue, or panic of the queue is full.
    pub fn push(&mut self, message: BrushPixelMessage) {
        assert!(self.data.len() < self.max_size);
        if let Some(m) = self
            .data
            .iter_mut()
            .find(|m| m.position == message.position)
        {
            if message.alpha > m.alpha {
                m.alpha = message.alpha;
                m.value = message.value;
            }
        } else {
            self.data.push_back(message);
        }
    }
}

/// Object to send to painting thread to control which textures are modified.
#[derive(Debug, Clone)]
pub struct TerrainTextureData {
    /// The height and width of the texture in pixels.
    pub chunk_size: Vector2<u32>,
    /// The kind of texture.
    pub kind: TerrainTextureKind,
    /// The texture resources, organized by chunk grid position.
    pub resources: FxHashMap<Vector2<i32>, TextureResource>,
}

/// Terrain textures come in multiple kinds.
/// Height textures contain f32 values for each vertex of the terrain.
/// Mask textures contain u8 values indicating transparency.
/// Coordinates are interpreted differently between the two kinds of texture
/// because the data in height textures overlaps with the data in neighboring chunks,
/// so the pixels along each edge are duplicated and must be kept in sync
/// so that the chunks do not disconnect.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum TerrainTextureKind {
    #[default]
    /// Height texture with f32 height values and overlapping edges between chunks.
    Height,
    /// Mask texture with u8 oppacity values.
    Mask,
}

/// Sender with methods for sending the messages which control a brush painting thread.
pub struct BrushSender(Sender<BrushThreadMessage>);

impl BrushSender {
    /// Create a new BrushSender using the given Sender.
    pub fn new(sender: Sender<BrushThreadMessage>) -> Self {
        Self(sender)
    }
    /// Begin a new stroke using the given brush.
    pub fn start_stroke(&self, brush: Brush, node: Handle<Node>, data: TerrainTextureData) {
        self.0
            .send(BrushThreadMessage::StartStroke(brush, node, data))
            .unwrap_or_else(on_send_failure);
    }
    /// End the current stroke.
    pub fn end_stroke(&self) {
        self.0
            .send(BrushThreadMessage::EndStroke)
            .unwrap_or_else(on_send_failure);
    }
    /// Draw a pixel using the brush that was set in the most recent call to [BrushSender::start_stroke].
    #[inline]
    pub fn draw_pixel(&self, position: Vector2<i32>, alpha: f32, value: f32) {
        if alpha == 0.0 {
            return;
        }
        self.0
            .send(BrushThreadMessage::Pixel(BrushPixelMessage {
                position,
                alpha,
                value,
            }))
            .unwrap_or_else(on_send_failure);
    }
}

fn on_send_failure(error: SendError<BrushThreadMessage>) {
    Log::err(format!(
        "A brush painting message was not sent. {:?}",
        error
    ));
}

/// Type for a callback that delivers the original data of textures that have been modified
/// by the brush so that changes might be undone.
pub type UndoChunkHandler = dyn FnMut(UndoData) + Send;

/// A record of original data data for chunks that have been modified by a brushstroke.
pub struct UndoData {
    /// The handle of the terrain being edited
    pub node: Handle<Node>,
    /// The data of the chunks as they were before the brushstroke.
    pub chunks: Vec<ChunkData>,
    /// The kind of data within the terrain that is being edited.
    pub target: BrushTarget,
}

#[derive(Default)]
/// Data for an in-progress terrain painting operation
pub struct BrushStroke {
    /// The brush that is currently being used. This determines how the terrain textures are edited.
    brush: Brush,
    /// The textures for the terrain that is currently being edited.
    textures: FxHashMap<Vector2<i32>, TextureResource>,
    /// The node of the terrain being edted
    node: Handle<Node>,
    /// Callback to handle the saved original chunk data after each stroke.
    /// This is called when [BrushThreadMessage::EndStroke] is received.
    undo_chunk_handler: Option<Box<UndoChunkHandler>>,
    /// A record of which pixels have been modified in each chunk since the last UpdateTextures.
    /// This is cleared after [BrushThreadMessage::UpdateTextures] is received,
    /// when the pixel data is transferred into the textures.
    chunks: StrokeChunks,
    /// Data copied from chunks that have been edited by the current brush stroke.
    /// This preserves the textures as they were before the stroke began, so that
    /// an undo command can be created at the end of the stroke.
    undo_chunks: Vec<ChunkData>,
    /// A record of every pixel of the stroke, including the strength of the brush at that pixel,
    /// the original value before the stroke began, and the current value.
    height_pixels: StrokeData<f32>,
    /// A record of every pixel of the stroke, including the strength of the brush at that pixel,
    /// the original value before the stroke began, and the current value.
    mask_pixels: StrokeData<u8>,
}

/// Stores pixels that have been modified by a brush during a stroke.
/// It remembers the strength of the brush, the value of the painted pixel,
/// and the value of the original pixel before the brushstroke.
/// This should be cleared after each stroke using [StrokeData::clear].
///
/// `V` is the type of data stored in the pixel being edited.
#[derive(Debug, Default)]
pub struct StrokeData<V>(FxHashMap<Vector2<i32>, StrokeElement<V>>);

/// A single pixel data of a brush stroke
#[derive(Debug, Copy, Clone)]
pub struct StrokeElement<V> {
    /// The intensity of the brush stroke, with 0.0 indicating a pixel that brush has not touched
    /// and 1.0 indicates a pixel fully covered by the brush.
    pub strength: f32,
    /// The value of the pixel before the stroke began.
    pub original_value: V,
    /// The current value of the pixel.
    pub latest_value: V,
}

impl BrushStroke {
    /// Create a BrushStroke with the given handler for saving undo data for chunks.
    pub fn with_chunk_handler(undo_chunk_handler: Box<UndoChunkHandler>) -> Self {
        Self {
            undo_chunk_handler: Some(undo_chunk_handler),
            ..Default::default()
        }
    }
    /// The brush that this stroke is using. This is immutable access only, because
    /// the brush's target may only be changed through [BrushStroke::start_stroke] or
    /// [BrushStroke::accept_messages].
    ///
    /// Mutable access to the brush's other properties is available through
    /// [BrushStroke::shape], [BrushStroke::mode], [BrushStroke::hardness],
    /// and [BrushStroke::alpha].
    pub fn brush(&self) -> &Brush {
        &self.brush
    }
    /// Mutable access to the brush's shape
    pub fn shape(&mut self) -> &mut BrushShape {
        &mut self.brush.shape
    }
    /// Mutable access to the brush's mode
    pub fn mode(&mut self) -> &mut BrushMode {
        &mut self.brush.mode
    }
    /// Mutable access to the brush's hardness. The hardness controls how the edges
    /// of the brush are blended with the original value of the texture.
    pub fn hardness(&mut self) -> &mut f32 {
        &mut self.brush.hardness
    }
    /// Mutable access to the brush's alpha. The alpha value controls how
    /// the operation's result is blended with the original value of the texture.
    pub fn alpha(&mut self) -> &mut f32 {
        &mut self.brush.alpha
    }
    /// Insert a stamp of the brush at the given position with the given texture scale and the given value.
    /// - `position`: The center of the stamp.
    /// - `scale`: The size of each pixel in 2D local space. This is used to convert the brush's shape from local space to texture space.
    /// - `value`: A value that is a parameter for the brush operation.
    pub fn stamp(&mut self, position: Vector2<f32>, scale: Vector2<f32>, value: f32) {
        let brush = self.brush.clone();
        brush.stamp(position, scale, |position, alpha| {
            self.draw_pixel(BrushPixelMessage {
                position,
                alpha,
                value,
            })
        });
    }
    /// Insert a smear of the brush at the given position with the given texture scale and the given value.
    /// - `start`: The center of the smear's start.
    /// - `end`: The center of the smear's end.
    /// - `scale`: The size of each pixel in 2D local space. This is used to convert the brush's shape from local space to texture space.
    /// - `value`: A value that is a parameter for the brush operation.
    pub fn smear(
        &mut self,
        start: Vector2<f32>,
        end: Vector2<f32>,
        scale: Vector2<f32>,
        value: f32,
    ) {
        let brush = self.brush.clone();
        brush.smear(start, end, scale, |position, alpha| {
            self.draw_pixel(BrushPixelMessage {
                position,
                alpha,
                value,
            })
        });
    }
    /// Prepare this object for a new brushstroke.
    pub fn clear(&mut self) {
        self.height_pixels.clear();
        self.mask_pixels.clear();
        self.chunks.clear();
    }
    /// Access the data in the textures to find the value for the pixel at the given position.
    pub fn data_pixel<V>(&self, position: Vector2<i32>) -> Option<V>
    where
        V: Clone,
    {
        // Determine which texture holds the data for the position.
        let grid_pos = self.chunks.pixel_position_to_grid_position(position);
        // Determine which pixel within the texture corresponds to the given position.
        let origin = self.chunks.chunk_to_origin(grid_pos);
        let p = position - origin;
        // Access the texture and extract the data.
        let texture = self.textures.get(&grid_pos)?;
        let index = self.chunks.pixel_index(p);
        let data = texture.data_ref();
        Some(data.data_of_type::<V>().unwrap()[index].clone())
    }
    /// Block on the given receiver until its messages are exhausted and perform the painting
    /// operations according to the messages.
    /// It does not return until the receiver's channel no longer has senders.
    pub fn accept_messages(&mut self, receiver: Receiver<BrushThreadMessage>) {
        let mut message_buffer = PixelMessageBuffer::new(MESSAGE_BUFFER_SIZE);
        loop {
            // Collect all waiting messages, until the buffer is full.
            while !message_buffer.is_full() {
                if let Ok(message) = receiver.try_recv() {
                    // Act on the message, potentially adding it to the message buffer.
                    self.handle_message(message, &mut message_buffer);
                } else {
                    break;
                }
            }
            if let Some(pixel) = message_buffer.pop() {
                // Perform the drawing operation for the current pixel message.
                self.handle_pixel_message(pixel);
            } else if self.chunks.count() > 0 {
                // We have run out of pixels to process, so before we block to wait for more,
                // write the currently processed pixels to the terrain textures.
                self.flush();
            } else {
                // If the message buffer is empty, we cannot proceed, so block until a message is available.
                // Block until either a message arrives or the channel is closed.
                if let Ok(message) = receiver.recv() {
                    // Act on the message, potentially adding it to the message buffer.
                    self.handle_message(message, &mut message_buffer);
                } else {
                    // The message buffer is empty and the channel is closed, so we're finished.
                    // Flush pixels to the textures.
                    self.end_stroke();
                    return;
                }
            }
        }
    }
    fn handle_message(
        &mut self,
        message: BrushThreadMessage,
        message_buffer: &mut PixelMessageBuffer,
    ) {
        match message {
            BrushThreadMessage::StartStroke(brush, node, textures) => {
                self.brush = brush;
                self.node = node;
                self.textures = textures.resources;
                self.chunks.set_layout(textures.kind, textures.chunk_size);
            }
            BrushThreadMessage::EndStroke => {
                // The stroke has ended, so finish processing all buffered pixel messages.
                while let Some(p) = message_buffer.pop() {
                    self.handle_pixel_message(p);
                }
                // Apply buffered pixels to the terrain textures.
                self.end_stroke();
            }
            BrushThreadMessage::Pixel(pixel) => {
                message_buffer.push(pixel);
            }
        }
    }
    /// -`brush`: The brush to paint with
    /// -`node`: The handle of the terrain being modified. It is not used except to pass to `undo_chunk_handler`,
    /// so it can be safely [Handle::NONE] if `undo_chunk_handler` is None, or if `undo_chunk_handler` is prepared for NONE.
    /// -`textures`: Hash map of texture resources that this stroke will edit.
    pub fn start_stroke(&mut self, brush: Brush, node: Handle<Node>, textures: TerrainTextureData) {
        self.brush = brush;
        self.node = node;
        self.chunks.set_layout(textures.kind, textures.chunk_size);
        self.textures = textures.resources;
    }
    /// Send the textures that have been touched by the brush to the undo handler,
    /// then write the current changes to the textures and clear the stroke to prepare
    /// for starting a new stroke.
    pub fn end_stroke(&mut self) {
        if let Some(handler) = &mut self.undo_chunk_handler {
            // Copy the textures that are about to be modified so that the modifications can be undone.
            self.chunks
                .copy_texture_data(&self.textures, &mut self.undo_chunks);
            // Send the saved textures to the handler so that an undo command might be created.
            handler(UndoData {
                node: self.node,
                chunks: std::mem::take(&mut self.undo_chunks),
                target: self.brush.target,
            });
        }
        // Flush pixels to the terrain textures
        self.apply();
        self.clear();
    }
    /// Insert a pixel with the given texture-space coordinates and strength.
    pub fn draw_pixel(&mut self, pixel: BrushPixelMessage) {
        let pixel = BrushPixelMessage {
            alpha: 0.5 * (1.0 - (pixel.alpha * std::f32::consts::PI).cos()),
            ..pixel
        };
        let position = pixel.position;
        match self.chunks.kind() {
            TerrainTextureKind::Height => self.accept_pixel_height(pixel),
            TerrainTextureKind::Mask => self.accept_pixel_mask(pixel),
        }
        self.chunks.write(position);
    }
    fn handle_pixel_message(&mut self, pixel: BrushPixelMessage) {
        self.draw_pixel(pixel);
        if self.chunks.count() >= PIXEL_BUFFER_SIZE {
            self.flush();
        }
    }
    fn smooth_height(
        &self,
        position: Vector2<i32>,
        kernel_radius: u32,
        original: f32,
        alpha: f32,
    ) -> f32 {
        let radius = kernel_radius as i32;
        let diameter = kernel_radius * 2 + 1;
        let area = (diameter * diameter - 1) as f32;
        let mut total = 0.0;
        for x in -radius..=radius {
            for y in -radius..=radius {
                if x == 0 && y == 0 {
                    continue;
                }
                let pos = position + Vector2::new(x, y);
                let value = self
                    .height_pixels
                    .original_pixel_value(pos)
                    .copied()
                    .or_else(|| self.data_pixel(position))
                    .unwrap_or_default();
                total += value;
            }
        }
        let smoothed = total / area;
        original * (1.0 - alpha) + smoothed * alpha
    }
    fn smooth_mask(
        &self,
        position: Vector2<i32>,
        kernel_radius: u32,
        original: u8,
        alpha: f32,
    ) -> u8 {
        let radius = kernel_radius as i32;
        let diameter = kernel_radius as u64 * 2 + 1;
        let area = diameter * diameter - 1;
        let mut total: u64 = 0;
        for x in -radius..=radius {
            for y in -radius..=radius {
                if x == 0 && y == 0 {
                    continue;
                }
                let pos = position + Vector2::new(x, y);
                let value = self
                    .mask_pixels
                    .original_pixel_value(pos)
                    .copied()
                    .or_else(|| self.data_pixel(position))
                    .unwrap_or_default();
                total += value as u64;
            }
        }
        let smoothed = total / area;
        (original as f32 * (1.0 - alpha) + smoothed as f32 * alpha).clamp(0.0, 255.0) as u8
    }
    fn accept_pixel_height(&mut self, pixel: BrushPixelMessage) {
        let position = pixel.position;
        let mut pixels = std::mem::take(&mut self.height_pixels);
        let original = pixels.update_strength(position, pixel.alpha, || self.data_pixel(position));
        self.height_pixels = pixels;
        let Some(original) = original else {
            return;
        };
        let alpha = self.brush.alpha * pixel.alpha;
        let result: f32 = match self.brush.mode {
            BrushMode::Raise { amount } => original + amount * alpha,
            BrushMode::Flatten => original * (1.0 - alpha) + pixel.value * alpha,
            BrushMode::Assign { value } => original * (1.0 - alpha) + value * alpha,
            BrushMode::Smooth { kernel_radius } => {
                self.smooth_height(position, kernel_radius, original, alpha)
            }
        };
        self.height_pixels.set_latest(position, result);
    }
    fn accept_pixel_mask(&mut self, pixel: BrushPixelMessage) {
        let position = pixel.position;
        let mut pixels = std::mem::take(&mut self.mask_pixels);
        let original = pixels.update_strength(position, pixel.alpha, || self.data_pixel(position));
        self.mask_pixels = pixels;
        let Some(original) = original else {
            return;
        };
        let alpha = self.brush.alpha * pixel.alpha;
        let result: u8 = match self.brush.mode {
            BrushMode::Raise { amount } => mask_raise(original, amount * alpha),
            BrushMode::Flatten => mask_lerp(original, pixel.value, alpha),
            BrushMode::Assign { value } => mask_lerp(original, value, alpha),
            BrushMode::Smooth { kernel_radius } => {
                self.smooth_mask(position, kernel_radius, original, alpha)
            }
        };
        self.mask_pixels.set_latest(position, result);
    }
    /// Update the texture resources to match the current state of this stroke.
    pub fn flush(&mut self) {
        // chunks stores the pixels that we have modified but not yet written to the textures.
        // If we have an undo handler, we must inform the handler of the textures we are writing to
        // because we are about to forget that we have written to them.
        if self.undo_chunk_handler.is_some() {
            // Copy the textures that are about to be modified so that the modifications can be undone.
            self.chunks
                .copy_texture_data(&self.textures, &mut self.undo_chunks);
        }
        // Do the actual texture modification.
        self.apply();
        // Erase our memory of having modified these pixels, since they have already been finalized
        // by writing them to the textures.
        self.chunks.clear();
    }
    fn apply(&self) {
        match self.chunks.kind() {
            TerrainTextureKind::Height => {
                self.chunks.apply(&self.height_pixels, &self.textures);
            }
            TerrainTextureKind::Mask => {
                self.chunks.apply(&self.mask_pixels, &self.textures);
            }
        }
    }
}

impl<V> StrokeData<V> {
    /// Reset the brush stroke so it is ready to begin a new stroke.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear()
    }
    /// For every pixel that is modified by the stroke, the original values
    /// is stored as it was before the stroke began.
    #[inline]
    pub fn original_pixel_value(&self, position: Vector2<i32>) -> Option<&V> {
        self.0.get(&position).map(|x| &x.original_value)
    }
    /// The updated pixel value based on whatever editing operation the stroke is performing.
    #[inline]
    pub fn latest_pixel_value(&self, position: Vector2<i32>) -> Option<&V> {
        self.0.get(&position).map(|x| &x.latest_value)
    }
    /// Update the stroke with a new value at the given pixel position.
    /// This must only be called after calling [StrokeData::update_strength]
    /// to ensure that this stroke contains data for the position.
    /// Otherwise this method may panic.
    #[inline]
    pub fn set_latest(&mut self, position: Vector2<i32>, value: V) {
        if let Some(el) = self.0.get_mut(&position) {
            el.latest_value = value;
        } else {
            panic!("Setting latest value of missing element");
        }
    }
    /// Stores or modifies the StrokeElement at the given position.
    /// If the element is updated, return the original pixel value of the element.
    /// - `position`: The position of the data to modify within the terrain.
    /// - `strength`: The strength of the brush at the position, from 0.0 to 1.0.
    /// The element is updated if the stored strength is less than the given strength.
    /// If there is no stored strength, that is treated as a strength of 0.0.
    /// - `pixel_value`: The current value of the data.
    /// This may be stored in the StrokeData if no pixel value is currently recorded for the given position.
    /// Otherwise, this value is ignored.
    #[inline]
    pub fn update_strength<F>(
        &mut self,
        position: Vector2<i32>,
        strength: f32,
        pixel_value: F,
    ) -> Option<V>
    where
        V: Clone,
        F: FnOnce() -> Option<V>,
    {
        if strength == 0.0 {
            None
        } else if let Some(element) = self.0.get_mut(&position) {
            if element.strength < strength {
                element.strength = strength;
                Some(element.original_value.clone())
            } else {
                None
            }
        } else {
            let value = pixel_value()?;
            let element = StrokeElement {
                strength,
                latest_value: value.clone(),
                original_value: value.clone(),
            };
            self.0.insert(position, element);
            Some(value)
        }
    }
}

/// Shape of a brush.
#[derive(Copy, Clone, Reflect, Debug)]
pub enum BrushShape {
    /// Circle with given radius.
    Circle {
        /// Radius of the circle.
        radius: f32,
    },
    /// Rectangle with given width and height.
    Rectangle {
        /// Width of the rectangle.
        width: f32,
        /// Length of the rectangle.
        length: f32,
    },
}

uuid_provider!(BrushShape = "a4dbfba0-077c-4658-9972-38384a8432f9");

impl Default for BrushShape {
    fn default() -> Self {
        BrushShape::Circle { radius: 1.0 }
    }
}

impl BrushShape {
    /// Return true if the given point is within the shape when positioned at the given center point.
    pub fn contains(&self, brush_center: Vector2<f32>, pixel_position: Vector2<f32>) -> bool {
        match *self {
            BrushShape::Circle { radius } => (brush_center - pixel_position).norm() < radius,
            BrushShape::Rectangle { width, length } => Rect::new(
                brush_center.x - width * 0.5,
                brush_center.y - length * 0.5,
                width,
                length,
            )
            .contains(pixel_position),
        }
    }
}

/// Paint mode of a brush. It defines operation that will be performed on the terrain.
#[derive(Clone, PartialEq, PartialOrd, Reflect, Debug)]
pub enum BrushMode {
    /// Raise or lower the value
    Raise {
        /// An offset to change the value by
        amount: f32,
    },
    /// Flattens value of the terrain data
    Flatten,
    /// Assigns a particular value to anywhere the brush touches.
    Assign {
        /// Fixed value to paint into the data
        value: f32,
    },
    /// Reduce sharp changes in the data.
    Smooth {
        /// Determines the size of each pixel's neighborhood in terms of
        /// distance from the pixel.
        /// 0 means no smoothing at all.
        /// 1 means taking the mean of the 3x3 square of pixels surrounding each smoothed pixel.
        /// 2 means using a 5x5 square of pixels. And so on.
        kernel_radius: u32,
    },
}

uuid_provider!(BrushMode = "48ad4cac-05f3-485a-b2a3-66812713841f");

impl Default for BrushMode {
    fn default() -> Self {
        BrushMode::Raise { amount: 1.0 }
    }
}

/// Paint target of a brush. It defines the data that the brush will operate on.
#[derive(Copy, Default, Clone, Reflect, Debug, PartialEq, Eq)]
pub enum BrushTarget {
    #[default]
    /// Modifies the height map
    HeightMap,
    /// Draws on a given layer
    LayerMask {
        /// The number of the layer to modify
        layer: usize,
    },
}

uuid_provider!(BrushTarget = "461c1be7-189e-44ee-b8fd-00b8fdbc668f");

/// Brush is used to modify terrain. It supports multiple shapes and modes.
#[derive(Clone, Reflect, Debug)]
pub struct Brush {
    /// Shape of the brush.
    pub shape: BrushShape,
    /// Paint mode of the brush.
    pub mode: BrushMode,
    /// The data to modify with the brush
    pub target: BrushTarget,
    /// Transform that can modify the shape of the brush
    pub transform: Matrix2<f32>,
    /// The softness of the edges of the brush.
    /// 0.0 means that the brush fades very gradually from opaque to transparent.
    /// 1.0 means that the edges of the brush do not fade.
    pub hardness: f32,
    /// The transparency of the brush, allowing the values beneath the brushstroke to show through.
    /// 0.0 means the brush is fully transparent and does not draw.
    /// 1.0 means the brush is fully opaque.
    pub alpha: f32,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            transform: Matrix2::identity(),
            hardness: 0.0,
            alpha: 1.0,
            shape: Default::default(),
            mode: Default::default(),
            target: Default::default(),
        }
    }
}

/// Verify that the brush operation is not so big that it could cause the editor to freeze.
/// The user can type in any size of brush they please, even disastrous sizes, and
/// this check prevents the editor from breaking.
fn within_size_limit(bounds: &Rect<i32>) -> bool {
    let size = bounds.size;
    let area = size.x * size.y;
    let accepted = area <= BRUSH_PIXEL_SANITY_LIMIT;
    if !accepted {
        Log::warn(format!(
            "Terrain brush operation dropped due to sanity limit: {}",
            area
        ))
    }
    accepted
}

impl Brush {
    /// Send the pixels for this brush to the brush thread.
    /// - `position`: The position of the brush in texture pixels.
    /// - `scale`: The size of each pixel in local 2D space. This is used
    /// to convert the brush's radius from local 2D to pixels.
    /// - `value`: The brush's value. The meaning of this number depends on the brush.
    /// - `draw_pixel`: The function that will draw the pixels to the terrain.
    pub fn stamp<F>(&self, position: Vector2<f32>, scale: Vector2<f32>, mut draw_pixel: F)
    where
        F: FnMut(Vector2<i32>, f32),
    {
        let mut transform = self.transform;
        let x_factor = scale.y / scale.x;
        transform.m11 *= x_factor;
        transform.m12 *= x_factor;
        match self.shape {
            BrushShape::Circle { radius } => {
                let iter = StampPixels::new(
                    CircleRaster(radius / scale.y),
                    position,
                    self.hardness,
                    transform,
                );
                if !within_size_limit(&iter.bounds()) {
                    return;
                }
                for BrushPixel { position, strength } in iter {
                    draw_pixel(position, strength);
                }
            }
            BrushShape::Rectangle { width, length } => {
                let iter = StampPixels::new(
                    RectRaster(width * 0.5 / scale.y, length * 0.5 / scale.y),
                    position,
                    self.hardness,
                    transform,
                );
                if !within_size_limit(&iter.bounds()) {
                    return;
                }
                for BrushPixel { position, strength } in iter {
                    draw_pixel(position, strength);
                }
            }
        }
    }
    /// Send the pixels for this brush to the brush thread.
    /// - `start`: The position of the brush when it started the smear in texture pixels.
    /// - `end`: The current position of the brush in texture pixels.
    /// - `scale`: The size of each pixel in local 2D space. This is used
    /// to convert the brush's radius from local 2D to pixels.
    /// - `draw_pixel`: The function that will draw the pixels to the terrain.
    pub fn smear<F>(
        &self,
        start: Vector2<f32>,
        end: Vector2<f32>,
        scale: Vector2<f32>,
        mut draw_pixel: F,
    ) where
        F: FnMut(Vector2<i32>, f32),
    {
        let mut transform = self.transform;
        let x_factor = scale.y / scale.x;
        transform.m11 *= x_factor;
        transform.m12 *= x_factor;
        match self.shape {
            BrushShape::Circle { radius } => {
                let iter = SmearPixels::new(
                    CircleRaster(radius / scale.y),
                    start,
                    end,
                    self.hardness,
                    transform,
                );
                if !within_size_limit(&iter.bounds()) {
                    return;
                }
                for BrushPixel { position, strength } in iter {
                    draw_pixel(position, strength);
                }
            }
            BrushShape::Rectangle { width, length } => {
                let iter = SmearPixels::new(
                    RectRaster(width * 0.5 / scale.y, length * 0.5 / scale.y),
                    start,
                    end,
                    self.hardness,
                    transform,
                );
                if !within_size_limit(&iter.bounds()) {
                    return;
                }
                for BrushPixel { position, strength } in iter {
                    draw_pixel(position, strength);
                }
            }
        }
    }
}

/// A copy of a layer of data from a chunk.
/// It can be height data or mask data, since the type is erased.
/// The layer that this data represents must be remembered externally.
pub struct ChunkData {
    /// The grid position of the original chunk.
    pub grid_position: Vector2<i32>,
    /// The size of the original chunk, to confirm that the chunk's size has not changed since the data was copied.
    pub size: Vector2<u32>,
    /// The type-erased data from either the height or one of the layers of the chunk.
    pub content: Box<[u8]>,
}

impl std::fmt::Debug for ChunkData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkData")
            .field("grid_position", &self.grid_position)
            .field("content", &format!("[..](len: {})", &self.content.len()))
            .finish()
    }
}

fn size_from_texture(texture: &ResourceDataRef<'_, Texture>) -> Vector2<u32> {
    match texture.kind() {
        crate::resource::texture::TextureKind::Rectangle { width, height } => {
            Vector2::new(width, height)
        }
        _ => unreachable!("Terrain texture was not rectangle."),
    }
}

impl ChunkData {
    /// Extract the size from the given texture and return true if that size matches
    /// the size required by this data. Log an error message and return false otherwise.
    fn verify_texture_size(&self, texture: &ResourceDataRef<'_, Texture>) -> bool {
        let size = size_from_texture(texture);
        if size != self.size {
            Log::err("Command swap failed due to texture size mismatch");
            false
        } else {
            true
        }
    }
    /// Create a ChunkData for the given texture at the given position.
    pub fn from_texture(grid_position: Vector2<i32>, texture: &TextureResource) -> Self {
        let data_ref = texture.data_ref();
        let size = size_from_texture(&data_ref);
        let data = Box::<[u8]>::from(data_ref.data());
        Self {
            grid_position,
            size,
            content: data,
        }
    }
    /// Swap the content of this data with the content of the given chunk's height map.
    pub fn swap_height(&mut self, chunk: &mut Chunk) {
        let mut data_ref = chunk.heightmap().data_ref();
        if !self.verify_texture_size(&data_ref) {
            return;
        }
        let mut modify = data_ref.modify();
        for (a, b) in modify.data_mut().iter_mut().zip(self.content.iter_mut()) {
            std::mem::swap(a, b);
        }
    }
    /// Swap the content of this data with the content of the given chunk's mask layer.
    pub fn swap_layer_mask(&mut self, chunk: &mut Chunk, layer: usize) {
        let mut data_ref = chunk.layer_masks[layer].data_ref();
        if !self.verify_texture_size(&data_ref) {
            return;
        }
        let mut modify = data_ref.modify();
        for (a, b) in modify.data_mut().iter_mut().zip(self.content.iter_mut()) {
            std::mem::swap(a, b);
        }
    }
    /// Swap the height data of the a chunk from the list with the height data in this object.
    /// The given list of chunks will be searched to find the chunk that matches `grid_position`.
    pub fn swap_height_from_list(&mut self, chunks: &mut [Chunk]) {
        for c in chunks {
            if c.grid_position == self.grid_position {
                self.swap_height(c);
                break;
            }
        }
    }
    /// Swap the layer mask data of a particular layer of a chunk from the list with the data in this object.
    /// The given list of chunks will be searched to find the chunk that matches `grid_position`.
    pub fn swap_layer_mask_from_list(&mut self, chunks: &mut [Chunk], layer: usize) {
        for c in chunks {
            if c.grid_position == self.grid_position {
                self.swap_layer_mask(c, layer);
                break;
            }
        }
    }
}
