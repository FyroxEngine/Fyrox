use std::{
    rc::Rc,
    collections::HashMap,
    fs::File,
    any::Any,
    path::Path,
    cell::RefCell,
    io::{
        Write,
        Read,
    },
    string::FromUtf8Error,
};
use byteorder::{
    ReadBytesExt,
    WriteBytesExt,
    LittleEndian,
};
use crate::{
    math::{
        vec3::Vec3,
        quat::Quat,
        mat4::Mat4,
    },
    utils::{
        pool::{
            Handle,
            Pool,
        }
    },
};

pub enum FieldKind {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    Vec3(Vec3),
    Quat(Quat),
    Mat4(Mat4),
    Data(Vec<u8>),
}

impl FieldKind {
    fn as_string(&self) -> String {
        match self {
            FieldKind::U8(data) => format!("<u8 = {}>, ", data),
            FieldKind::I8(data) => format!("<i8 = {}>, ", data),
            FieldKind::U16(data) => format!("<u16 = {}>, ", data),
            FieldKind::I16(data) => format!("<i16 = {}>, ", data),
            FieldKind::U32(data) => format!("<u32 = {}>, ", data),
            FieldKind::I32(data) => format!("<i32 = {}>, ", data),
            FieldKind::U64(data) => format!("<u64 = {}>, ", data),
            FieldKind::I64(data) => format!("<i64 = {}>, ", data),
            FieldKind::F32(data) => format!("<f32 = {}>, ", data),
            FieldKind::F64(data) => format!("<f64 = {}>, ", data),
            FieldKind::Vec3(data) => {
                format!("<vec3 = {}; {}; {}>, ", data.x, data.y, data.z)
            }
            FieldKind::Quat(data) => {
                format!("<quat = {}; {}; {}; {}>, ", data.x, data.y, data.z, data.w)
            }
            FieldKind::Mat4(data) => {
                // Lazy
                format!("<mat4>, ")
            }
            FieldKind::Data(data) => {
                base64::encode(data)
            }
        }
    }
}

trait FieldData {
    fn read(&mut self, kind: &FieldKind) -> VisitResult;
    fn write(&self) -> FieldKind;
}

macro_rules! impl_field_data (($type_name:ty, $($kind:tt)*) => {
    impl FieldData for $type_name {
        fn read(&mut self, kind: &FieldKind) -> VisitResult {
            match kind {
                $($kind)*(data) => {
                    *self = data.clone();
                    Ok(())
                },
                _ => Err(VisitError::FieldTypeDoesNotMatch)
            }
        }

        fn write(&self) -> FieldKind {
             $($kind)*(self.clone())
        }
    }
});

impl_field_data!(u64, FieldKind::U64);
impl_field_data!(i64, FieldKind::I64);
impl_field_data!(u32, FieldKind::U32);
impl_field_data!(i32, FieldKind::I32);
impl_field_data!(u16, FieldKind::U16);
impl_field_data!(i16, FieldKind::I16);
impl_field_data!(u8, FieldKind::U8);
impl_field_data!(i8, FieldKind::I8);
impl_field_data!(f32, FieldKind::F32);
impl_field_data!(f64, FieldKind::F64);
impl_field_data!(Vec3, FieldKind::Vec3);
impl_field_data!(Quat, FieldKind::Quat);
impl_field_data!(Mat4, FieldKind::Mat4);
impl_field_data!(Vec<u8>, FieldKind::Data);

pub struct Field {
    name: String,
    kind: FieldKind,
}

#[derive(Debug)]
pub enum VisitError {
    Io(std::io::Error),
    UnknownFieldType(u8),
    FieldDoesNotExist(String),
    FieldAlreadyExists(String),
    RegionAlreadyExists(String),
    InvalidCurrentNode,
    FieldTypeDoesNotMatch,
    RegionDoesNotExist(String),
    NoActiveNode,
    NotSupportedFormat,
    InvalidName,
    TypeMismatch,
    User(String),
}

impl From<std::io::Error> for VisitError {
    fn from(io_err: std::io::Error) -> Self {
        VisitError::Io(io_err)
    }
}

impl From<FromUtf8Error> for VisitError {
    fn from(_: FromUtf8Error) -> Self {
        VisitError::InvalidName
    }
}

pub type VisitResult = Result<(), VisitError>;

impl Field {
    pub fn new(name: &str, kind: FieldKind) -> Self {
        Self {
            name: name.to_owned(),
            kind,
        }
    }

    fn save(field: &Field, file: &mut File) -> VisitResult {
        let name = field.name.as_bytes();
        file.write_u32::<LittleEndian>(name.len() as u32)?;
        file.write(name)?;
        match &field.kind {
            FieldKind::U8(data) => {
                file.write_u8(1)?;
                file.write_u8(*data)?;
            }
            FieldKind::I8(data) => {
                file.write_i8(2)?;
                file.write_i8(*data)?;
            }
            FieldKind::U16(data) => {
                file.write_u8(3)?;
                file.write_u16::<LittleEndian>(*data)?;
            }
            FieldKind::I16(data) => {
                file.write_u8(4)?;
                file.write_i16::<LittleEndian>(*data)?;
            }
            FieldKind::U32(data) => {
                file.write_u8(5)?;
                file.write_u32::<LittleEndian>(*data)?;
            }
            FieldKind::I32(data) => {
                file.write_u8(6)?;
                file.write_i32::<LittleEndian>(*data)?;
            }
            FieldKind::U64(data) => {
                file.write_u8(7)?;
                file.write_u64::<LittleEndian>(*data)?;
            }
            FieldKind::I64(data) => {
                file.write_u8(8)?;
                file.write_i64::<LittleEndian>(*data)?;
            }
            FieldKind::F32(data) => {
                file.write_u8(9)?;
                file.write_f32::<LittleEndian>(*data)?;
            }
            FieldKind::F64(data) => {
                file.write_u8(10)?;
                file.write_f64::<LittleEndian>(*data)?;
            }
            FieldKind::Vec3(data) => {
                file.write_u8(11)?;
                file.write_f32::<LittleEndian>(data.x)?;
                file.write_f32::<LittleEndian>(data.y)?;
                file.write_f32::<LittleEndian>(data.z)?;
            }
            FieldKind::Quat(data) => {
                file.write_u8(12)?;
                file.write_f32::<LittleEndian>(data.x)?;
                file.write_f32::<LittleEndian>(data.y)?;
                file.write_f32::<LittleEndian>(data.z)?;
                file.write_f32::<LittleEndian>(data.w)?;
            }
            FieldKind::Mat4(data) => {
                file.write_u8(13)?;
                for f in &data.f {
                    file.write_f32::<LittleEndian>(*f)?;
                }
            }
            FieldKind::Data(data) => {
                file.write_u8(14)?;
                file.write_u32::<LittleEndian>(data.len() as u32)?;
                file.write(data.as_slice());
            }
        }
        Ok(())
    }

    fn load(file: &mut File) -> Result<Field, VisitError> {
        let name_len = file.read_u32::<LittleEndian>()? as usize;
        let mut raw_name = Vec::new();
        for _ in 0..name_len {
            raw_name.push(file.read_u8()?);
        }
        let id = file.read_u8()?;
        Ok(Field::new(String::from_utf8(raw_name)?.as_str(), match id {
            1 => FieldKind::U8(file.read_u8()?),
            2 => FieldKind::I8(file.read_i8()?),
            3 => FieldKind::U16(file.read_u16::<LittleEndian>()?),
            4 => FieldKind::I16(file.read_i16::<LittleEndian>()?),
            5 => FieldKind::U32(file.read_u32::<LittleEndian>()?),
            6 => FieldKind::I32(file.read_i32::<LittleEndian>()?),
            7 => FieldKind::U64(file.read_u64::<LittleEndian>()?),
            8 => FieldKind::I64(file.read_i64::<LittleEndian>()?),
            9 => FieldKind::F32(file.read_f32::<LittleEndian>()?),
            10 => FieldKind::F64(file.read_f64::<LittleEndian>()?),
            11 => FieldKind::Vec3({
                let x = file.read_f32::<LittleEndian>()?;
                let y = file.read_f32::<LittleEndian>()?;
                let z = file.read_f32::<LittleEndian>()?;
                Vec3 { x, y, z }
            }),
            12 => FieldKind::Quat({
                let x = file.read_f32::<LittleEndian>()?;
                let y = file.read_f32::<LittleEndian>()?;
                let z = file.read_f32::<LittleEndian>()?;
                let w = file.read_f32::<LittleEndian>()?;
                Quat { x, y, z, w }
            }),
            13 => FieldKind::Mat4({
                let mut f = [0.0f32; 16];
                for n in &mut f {
                    *n = file.read_f32::<LittleEndian>()?;
                }
                Mat4 { f }
            }),
            14 => FieldKind::Data({
                let len = file.read_u32::<LittleEndian>()? as usize;
                let mut vec = Vec::with_capacity(len);
                for _ in 0..len {
                    vec.push(file.read_u8()?);
                }
                vec
            }),
            _ => return Err(VisitError::UnknownFieldType(id))
        }))
    }

    fn as_string(&self) -> String {
        format!("{}{}", self.name, self.kind.as_string())
    }
}

pub struct Node {
    name: String,
    fields: Vec<Field>,
    parent: Handle<Node>,
    children: Vec<Handle<Node>>,
}

impl Node {
    fn new(name: &str, parent: Handle<Node>) -> Self {
        Self {
            name: name.to_owned(),
            fields: Vec::new(),
            parent,
            children: Vec::new(),
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Self {
            name: String::new(),
            fields: Vec::new(),
            parent: Handle::none(),
            children: Vec::new(),
        }
    }
}

pub struct Visitor {
    nodes: Pool<Node>,
    rc_map: HashMap<u64, Rc<dyn Any>>,
    reading: bool,
    current_node: Handle<Node>,
    root: Handle<Node>,
}

pub trait Visit {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult;
}

macro_rules! impl_generic_visit (($name:ident, $type_name:ty) => {
    pub fn $name(&mut self, name: &str, value: &mut $type_name) -> VisitResult {
        self.visit_generic(name, value)
    }
});

impl Visitor {
    const MAGIC: &'static str = "RG3D";

    fn new() -> Self {
        let mut nodes = Pool::new();
        let root = nodes.spawn(Node::new("__ROOT__", Handle::none()));
        Self {
            nodes,
            rc_map: HashMap::new(),
            reading: false,
            current_node: root.clone(),
            root,
        }
    }

    fn find_field(&mut self, name: &str) -> Option<&mut Field> {
        if let Some(node) = self.nodes.borrow_mut(&self.current_node) {
            for field in node.fields.iter_mut() {
                if field.name == name {
                    return Some(field);
                }
            }
        }
        None
    }

    fn current_node(&mut self) -> Option<&mut Node> {
        self.nodes.borrow_mut(&self.current_node)
    }

    fn visit_generic<T>(&mut self, name: &str, value: &mut T) -> VisitResult
        where T: FieldData {
        if self.reading {
            if let Some(field) = self.find_field(name) {
                value.read(&field.kind)
            } else {
                Err(VisitError::FieldDoesNotExist(name.to_owned()))
            }
        } else {
            if let Some(_) = self.find_field(name) {
                Err(VisitError::FieldAlreadyExists(name.to_owned()))
            } else {
                if let Some(node) = self.current_node() {
                    node.fields.push(Field::new(name, value.write()));
                    Ok(())
                } else {
                    Err(VisitError::NoActiveNode)
                }
            }
        }
    }

    impl_generic_visit!(visit_u64, u64);
    impl_generic_visit!(visit_i64, i64);
    impl_generic_visit!(visit_u32, u32);
    impl_generic_visit!(visit_i32, i32);
    impl_generic_visit!(visit_u16, u16);
    impl_generic_visit!(visit_i16, i16);
    impl_generic_visit!(visit_u8, u8);
    impl_generic_visit!(visit_i8, i8);
    impl_generic_visit!(visit_f32, f32);
    impl_generic_visit!(visit_f64, f64);
    impl_generic_visit!(visit_vec3, Vec3);
    impl_generic_visit!(visit_quat, Quat);
    impl_generic_visit!(visit_mat4, Mat4);
    impl_generic_visit!(visit_data, Vec<u8>);

    pub fn enter_region(&mut self, name: &str) -> VisitResult {
        if self.reading {
            if let Some(node) = self.nodes.borrow(&self.current_node) {
                let mut region = Handle::none();
                for child_handle in node.children.iter() {
                    if let Some(child) = self.nodes.borrow(child_handle) {
                        if child.name == name {
                            region = child_handle.clone();
                            break;
                        }
                    }
                }
                if region.is_some() {
                    self.current_node = region;
                    Ok(())
                } else {
                    Err(VisitError::RegionDoesNotExist(name.to_owned()))
                }
            } else {
                Err(VisitError::NoActiveNode)
            }
        } else {
            // Make sure that node does not exists already.
            if let Some(node) = self.nodes.borrow(&self.current_node) {
                for child_handle in node.children.iter() {
                    if let Some(child) = self.nodes.borrow(child_handle) {
                        if child.name == name {
                            return Err(VisitError::RegionAlreadyExists(name.to_owned()));
                        }
                    }
                }
            }

            let node_handle = self.nodes.spawn(Node::new(name, self.current_node.clone()));
            if let Some(node) = self.nodes.borrow_mut(&self.current_node.clone()) {
                node.children.push(node_handle.clone());
            }
            self.current_node = node_handle;

            Ok(())
        }
    }

    pub fn leave_region(&mut self) -> VisitResult {
        self.current_node = if let Some(node) = self.nodes.borrow(&self.current_node) {
            node.parent.clone()
        } else {
            return Err(VisitError::NoActiveNode);
        };
        Ok(())
    }

    pub fn visit_rc<T>(&mut self, name: &str, rc: &mut Rc<T>) -> VisitResult
        where T: Visit + 'static {
        self.enter_region(name)?;

        if self.reading {
            let mut raw = 0;
            self.visit_u64("Id", &mut raw)?;
            if let Some(ptr) = self.rc_map.get(&raw) {
                if let Ok(res) = Rc::downcast::<T>(ptr.clone()) {
                    *rc = res;
                } else {
                    return Err(VisitError::TypeMismatch);
                }
            } else {
                self.rc_map.insert(raw as u64, rc.clone());

                // Deserialize inner data.
                let raw = Rc::into_raw(rc.clone()) as *const T as *mut T;
                unsafe { &mut *raw }.visit("Data", self)?;
                unsafe { Rc::from_raw(raw) };
            }
        } else {
            let raw = Rc::into_raw(rc.clone()) as *const T as *mut T;
            unsafe { Rc::from_raw(raw); };
            let mut index = raw as u64;
            self.visit_u64("Id", &mut index)?;

            if !self.rc_map.contains_key(&index) {
                self.rc_map.insert(index, rc.clone());
                // Serialize inner data.
                unsafe { &mut *raw }.visit("Data", self)?;
            }
        }

        self.leave_region()?;

        Ok(())
    }

    pub fn visit_vec<T>(&mut self, name: &str, vec: &mut Vec<T>) -> VisitResult
        where T: Default + Visit + 'static {
        self.enter_region(name)?;

        let mut len = vec.len() as u32;
        self.visit_u32("Length", &mut len)?;

        if self.reading {
            for index in 0..len {
                let region_name = format!("Item{}", index);
                self.enter_region(region_name.as_str())?;
                let mut object = T::default();
                object.visit("Data", self)?;
                vec.push(object);
                self.leave_region()?;
            }
        } else {
            for (index, item) in vec.iter_mut().enumerate() {
                let region_name = format!("Item{}", index);
                self.enter_region(region_name.as_str())?;
                item.visit("Data", self)?;
                self.leave_region()?;
            }
        }
        self.leave_region()?;
        Ok(())
    }

    //pub fn visit_string(&mut self, name: &str, string: &mut String) -> VisitResult {

    //}

    pub fn visit_option<T>(&mut self, name: &str, opt: &mut Option<T>) -> VisitResult
        where T: Default + Visit + 'static {
        self.enter_region(name)?;

        let mut is_some = if opt.is_some() { 1 } else { 0 };
        self.visit_u8("IsSome", &mut is_some)?;

        if is_some != 0 {
            if self.reading {
                let mut value = T::default();
                value.visit("Data", self)?;
                *opt = Some(value);
            } else {
                opt.as_mut().unwrap().visit("Data", self)?;
            }
        }

        self.leave_region()?;
        Ok(())
    }

    fn print_node(&self, node_handle: &Handle<Node>, nesting: usize, out_string: &mut String) {
        let offset = (0..nesting).map(|_| { "\t" }).collect::<String>();
        if let Some(node) = self.nodes.borrow(&node_handle) {
            *out_string += format!("{}{}[Fields={}, Children={}]: ", offset, node.name, node.fields.len(), node.children.len()).as_str();
            for field in node.fields.iter() {
                *out_string += field.name.as_str();

                match &field.kind {
                    FieldKind::U8(data) => *out_string += format!("<u8 = {}>, ", data).as_str(),
                    FieldKind::I8(data) => *out_string += format!("<i8 = {}>, ", data).as_str(),
                    FieldKind::U16(data) => *out_string += format!("<u16 = {}>, ", data).as_str(),
                    FieldKind::I16(data) => *out_string += format!("<i16 = {}>, ", data).as_str(),
                    FieldKind::U32(data) => *out_string += format!("<u32 = {}>, ", data).as_str(),
                    FieldKind::I32(data) => *out_string += format!("<i32 = {}>, ", data).as_str(),
                    FieldKind::U64(data) => *out_string += format!("<u64 = {}>, ", data).as_str(),
                    FieldKind::I64(data) => *out_string += format!("<i64 = {}>, ", data).as_str(),
                    FieldKind::F32(data) => *out_string += format!("<f32 = {}>, ", data).as_str(),
                    FieldKind::F64(data) => *out_string += format!("<f64 = {}>, ", data).as_str(),
                    FieldKind::Vec3(data) => {
                        *out_string += format!("<vec3 = {}; {}; {}>, ", data.x, data.y, data.z).as_str()
                    }
                    FieldKind::Quat(data) => {
                        *out_string += format!("<quat = {}; {}; {}; {}>, ", data.x, data.y, data.z, data.w).as_str()
                    }
                    FieldKind::Mat4(data) => {
                        // Lazy
                        *out_string += format!("<mat4>, ").as_str()
                    }
                    FieldKind::Data(data) => {
                        *out_string += base64::encode(data).as_str();
                    }
                }
            }

            *out_string += format!("\n").as_str();

            for child_handle in node.children.iter() {
                self.print_node(child_handle, nesting + 1, out_string);
            }
        }
    }

    pub fn save_text(&self) -> String {
        let mut out_string = String::new();
        self.print_node(&self.root.clone(), 0, &mut out_string);
        out_string
    }

    pub fn save_binary(&self, path: &Path) -> VisitResult {
        let mut file = File::create(path)?;
        file.write(Self::MAGIC.as_bytes())?;
        let mut stack = Vec::new();
        stack.push(self.root.clone());
        while let Some(node_handle) = stack.pop() {
            if let Some(node) = self.nodes.borrow(&node_handle) {
                let name = node.name.as_bytes();
                file.write_u32::<LittleEndian>(name.len() as u32)?;
                file.write(name)?;

                file.write_u32::<LittleEndian>(node.fields.len() as u32)?;
                for field in node.fields.iter() {
                    Field::save(field, &mut file)?
                }

                file.write_u32::<LittleEndian>(node.children.len() as u32)?;
                for child_handle in node.children.iter() {
                    stack.push(child_handle.clone());
                }
            }
        }
        Ok(())
    }

    fn load_node_binary(&mut self, file: &mut File) -> Result<Handle<Node>, VisitError> {
        let name_len = file.read_u32::<LittleEndian>()? as usize;
        let mut raw_name = Vec::new();
        for _ in 0..name_len {
            raw_name.push(file.read_u8()?);
        }

        let mut node = Node::default();
        node.name = String::from_utf8(raw_name)?;
        println!("{}", node.name);

        let field_count = file.read_u32::<LittleEndian>()? as usize;
        for _ in 0..field_count {
            let field = Field::load(file)?;
            println!("Field: {}", field.as_string());
            node.fields.push(field);
        }

        let mut children = Vec::new();
        let child_count = file.read_u32::<LittleEndian>()? as usize;
        for _ in 0..child_count {
            children.push(self.load_node_binary(file)?);
        }

        node.children = children.clone();

        let handle = self.nodes.spawn(node);
        for child_handle in children.iter() {
            if let Some(child) = self.nodes.borrow_mut(child_handle) {
                child.parent = handle.clone();
            }
        }

        Ok(handle)
    }

    pub fn load_binary(path: &Path) -> Result<Self, VisitError> {
        let mut file = File::open(path)?;
        let mut magic: [u8; 4] = Default::default();
        file.read(&mut magic)?;
        if !magic.eq(Self::MAGIC.as_bytes()) {
            return Err(VisitError::NotSupportedFormat);
        }
        let mut visitor = Self {
            nodes: Pool::new(),
            rc_map: Default::default(),
            reading: true,
            current_node: Handle::none(),
            root: Handle::none(),
        };
        visitor.root = visitor.load_node_binary(&mut file)?;
        visitor.current_node = visitor.root.clone();
        Ok(visitor)
    }
}

impl<T> Visit for RefCell<T> where T: Visit + 'static {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.borrow_mut().visit(name, visitor)
    }
}

impl<T> Visit for Rc<T> where T: Default + Visit + 'static {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.visit_rc(name, self)
    }
}

#[cfg(test)]
mod test {
    use std::{
        rc::Rc,
        path::Path,
    };
    use crate::utils::visitor::{Visitor, Visit, VisitResult, VisitError};
    use std::fs::File;
    use std::io::Write;

    pub struct Model {
        data: u64
    }

    pub struct Texture {
        data: Vec<u8>
    }

    impl Visit for Texture {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            visitor.enter_region(name)?;
            visitor.visit_data("Data", &mut self.data)?;
            visitor.leave_region()
        }
    }

    impl Visit for Model {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            visitor.enter_region(name)?;
            visitor.visit_u64("Data", &mut self.data)?;
            visitor.leave_region()
        }
    }

    impl Visit for ResourceKind {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            match self {
                ResourceKind::Unknown => Err(VisitError::User(format!("invalid resource type"))),
                ResourceKind::Texture(tex) => tex.visit(name, visitor),
                ResourceKind::Model(model) => model.visit(name, visitor)
            }
        }
    }

    pub enum ResourceKind {
        Unknown,
        Model(Model),
        Texture(Texture),
    }

    struct Resource {
        kind: ResourceKind,
        data: u16,
    }

    impl Resource {
        fn new(kind: ResourceKind) -> Self {
            Self {
                kind,
                data: 0,
            }
        }
    }

    impl Default for Resource {
        fn default() -> Self {
            Self {
                kind: ResourceKind::Unknown,
                data: 0,
            }
        }
    }

    impl Visit for Resource {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            if visitor.reading {} else {
                let mut kind_id = match &self.kind {
                    ResourceKind::Unknown => return Err(VisitError::User(format!("Invalid resource!"))),
                    ResourceKind::Model(model) => 0,
                    ResourceKind::Texture(texture) => 1
                };

                visitor.visit_u8("KindId", &mut kind_id)?;
                self.kind.visit("KindData", visitor)?;
            }
            visitor.visit_u16("ResData", &mut self.data)
        }
    }

    struct Foo {
        bar: u64,
        shared_resource: Option<Rc<Resource>>,
    }

    impl Default for Foo {
        fn default() -> Self {
            Self {
                bar: 0,
                shared_resource: None,
            }
        }
    }

    impl Foo {
        fn new(resource: Rc<Resource>) -> Self {
            Self {
                bar: 123,
                shared_resource: Some(resource),
            }
        }
    }

    impl Visit for Foo {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            visitor.visit_u64("Bar", &mut self.bar)?;
            visitor.visit_option("SharedResource", &mut self.shared_resource)?;
            Ok(())
        }
    }


    fn visitor_save_test() {
        let mut visitor = Visitor::new();
        let mut resource = Rc::new(Resource::new(ResourceKind::Model(Model { data: 555 })));
        visitor.visit_rc("SharedResource", &mut resource).unwrap();

        let mut objects = vec![
            Foo::new(resource.clone()),
            Foo::new(resource)
        ];

        visitor.visit_vec("Objects", &mut objects).unwrap();

        visitor.save_binary(Path::new("test.bin")).unwrap();
        if let Ok(mut file) = File::create(Path::new("test.txt")) {
            file.write(visitor.save_text().as_bytes()).unwrap();
        }
    }

    #[test]
    fn visitor_load_test() {
        let mut visitor = Visitor::load_binary(Path::new("test.bin")).unwrap();
        let mut resource: Rc<Resource> = Rc::new(Default::default());
        visitor.visit_rc("SharedResource", &mut resource).unwrap();

        let mut objects: Vec<Foo> = Vec::new();
        visitor.visit_vec("Objects", &mut objects).unwrap();
    }
}