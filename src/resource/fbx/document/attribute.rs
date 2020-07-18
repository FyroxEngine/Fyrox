use std::fmt::Formatter;

pub enum FbxAttribute {
    Double(f64),
    Float(f32),
    Integer(i32),
    Long(i64),
    Bool(bool),
    String(String), // ASCII Fbx always have every attribute in string form
}

impl std::fmt::Display for FbxAttribute {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            FbxAttribute::Double(double) => write!(f, "{}", double),
            FbxAttribute::Float(float) => write!(f, "{}", float),
            FbxAttribute::Integer(integer) => write!(f, "{}", integer),
            FbxAttribute::Long(long) => write!(f, "{}", long),
            FbxAttribute::Bool(boolean) => write!(f, "{}", boolean),
            FbxAttribute::String(string) => write!(f, "{}", string),
        }
    }
}

impl FbxAttribute {
    pub fn as_i32(&self) -> Result<i32, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val as i32),
            FbxAttribute::Float(val) => Ok(*val as i32),
            FbxAttribute::Integer(val) => Ok(*val),
            FbxAttribute::Long(val) => Ok(*val as i32),
            FbxAttribute::Bool(val) => Ok(*val as i32),
            FbxAttribute::String(val) => match lexical::parse::<i32, _>(val.as_str()) {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {} to i32", val)),
            },
        }
    }

    pub fn as_i64(&self) -> Result<i64, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val as i64),
            FbxAttribute::Float(val) => Ok(*val as i64),
            FbxAttribute::Integer(val) => Ok(i64::from(*val)),
            FbxAttribute::Long(val) => Ok(*val as i64),
            FbxAttribute::Bool(val) => Ok(*val as i64),
            FbxAttribute::String(val) => match lexical::parse::<i64, _>(val.as_str()) {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {} to i64", val)),
            },
        }
    }

    pub fn as_f64(&self) -> Result<f64, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val),
            FbxAttribute::Float(val) => Ok(f64::from(*val)),
            FbxAttribute::Integer(val) => Ok(f64::from(*val)),
            FbxAttribute::Long(val) => Ok(*val as f64),
            FbxAttribute::Bool(val) => Ok((*val as i64) as f64),
            FbxAttribute::String(val) => match lexical::parse_lossy::<f64, _>(val.as_str()) {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {} to f64", val)),
            },
        }
    }

    pub fn as_f32(&self) -> Result<f32, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val as f32),
            FbxAttribute::Float(val) => Ok(*val),
            FbxAttribute::Integer(val) => Ok(*val as f32),
            FbxAttribute::Long(val) => Ok(*val as f32),
            FbxAttribute::Bool(val) => Ok((*val as i32) as f32),
            FbxAttribute::String(val) => match lexical::parse_lossy::<f32, _>(val.as_str()) {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {} to f32", val)),
            },
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            FbxAttribute::Double(val) => val.to_string(),
            FbxAttribute::Float(val) => val.to_string(),
            FbxAttribute::Integer(val) => val.to_string(),
            FbxAttribute::Long(val) => val.to_string(),
            FbxAttribute::Bool(val) => val.to_string(),
            FbxAttribute::String(val) => val.clone(),
        }
    }
}
