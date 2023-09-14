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
            Self::Double(double) => write!(f, "{}", double),
            Self::Float(float) => write!(f, "{}", float),
            Self::Integer(integer) => write!(f, "{}", integer),
            Self::Long(long) => write!(f, "{}", long),
            Self::Bool(boolean) => write!(f, "{}", boolean),
            Self::String(string) => write!(f, "{}", string),
        }
    }
}

impl FbxAttribute {
    pub fn as_i32(&self) -> Result<i32, String> {
        Ok(match self {
            Self::Double(val) => *val as i32,
            Self::Float(val) => *val as i32,
            Self::Integer(val) => *val,
            Self::Long(val) => *val as i32,
            Self::Bool(val) => *val as i32,
            Self::String(val) => match val.parse::<i32>() {
                Ok(i) => i,
                Err(_) => return Err(format!("Unable to convert string {} to i32", val)),
            },
        })
    }

    pub fn as_i64(&self) -> Result<i64, String> {
        Ok(match self {
            Self::Double(val) => *val as i64,
            Self::Float(val) => *val as i64,
            Self::Integer(val) => i64::from(*val),
            Self::Long(val) => *val,
            Self::Bool(val) => *val as i64,
            Self::String(val) => match val.parse::<i64>() {
                Ok(i) => i,
                Err(_) => return Err(format!("Unable to convert string {} to i64", val)),
            },
        })
    }

    pub fn as_f64(&self) -> Result<f64, String> {
        Ok(match self {
            Self::Double(val) => *val,
            Self::Float(val) => f64::from(*val),
            Self::Integer(val) => f64::from(*val),
            Self::Long(val) => *val as f64,
            Self::Bool(val) => (*val as i64) as f64,
            Self::String(val) => match val.parse::<f64>() {
                Ok(i) => i,
                Err(_) => return Err(format!("Unable to convert string {} to f64", val)),
            },
        })
    }

    pub fn as_f32(&self) -> Result<f32, String> {
        Ok(match self {
            Self::Double(val) => *val as f32,
            Self::Float(val) => *val,
            Self::Integer(val) => *val as f32,
            Self::Long(val) => *val as f32,
            Self::Bool(val) => (*val as i32) as f32,
            Self::String(val) => match val.parse::<f32>() {
                Ok(i) => i,
                Err(_) => return Err(format!("Unable to convert string {} to f32", val)),
            },
        })
    }

    pub fn as_string(&self) -> String {
        match self {
            Self::Double(val) => val.to_string(),
            Self::Float(val) => val.to_string(),
            Self::Integer(val) => val.to_string(),
            Self::Long(val) => val.to_string(),
            Self::Bool(val) => val.to_string(),
            Self::String(val) => val.clone(),
        }
    }
}
