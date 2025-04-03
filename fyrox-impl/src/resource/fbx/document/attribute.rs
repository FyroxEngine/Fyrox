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

use std::fmt::Formatter;

pub enum FbxAttribute {
    Double(f64),
    Float(f32),
    Integer(i32),
    Long(i64),
    Bool(bool),
    String(String), // ASCII Fbx always have every attribute in string form
    RawData(Vec<u8>),
}

impl std::fmt::Display for FbxAttribute {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Self::Double(double) => write!(f, "{double}"),
            Self::Float(float) => write!(f, "{float}"),
            Self::Integer(integer) => write!(f, "{integer}"),
            Self::Long(long) => write!(f, "{long}"),
            Self::Bool(boolean) => write!(f, "{boolean}"),
            Self::String(string) => write!(f, "{string}"),
            Self::RawData(raw) => write!(f, "{raw:?}"),
        }
    }
}

impl FbxAttribute {
    pub fn as_i32(&self) -> Result<i32, String> {
        match self {
            Self::Double(val) => Ok(*val as i32),
            Self::Float(val) => Ok(*val as i32),
            Self::Integer(val) => Ok(*val),
            Self::Long(val) => Ok(*val as i32),
            Self::Bool(val) => Ok(*val as i32),
            Self::String(val) => match val.parse::<i32>() {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {val} to i32")),
            },
            Self::RawData(_) => Err("Unable to convert raw data to i32".to_string()),
        }
    }

    pub fn as_i64(&self) -> Result<i64, String> {
        match self {
            Self::Double(val) => Ok(*val as i64),
            Self::Float(val) => Ok(*val as i64),
            Self::Integer(val) => Ok(i64::from(*val)),
            Self::Long(val) => Ok(*val),
            Self::Bool(val) => Ok(*val as i64),
            Self::String(val) => match val.parse::<i64>() {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {val} to i64")),
            },
            Self::RawData(_) => Err("Unable to convert raw data to i64".to_string()),
        }
    }

    pub fn as_f64(&self) -> Result<f64, String> {
        match self {
            Self::Double(val) => Ok(*val),
            Self::Float(val) => Ok(f64::from(*val)),
            Self::Integer(val) => Ok(f64::from(*val)),
            Self::Long(val) => Ok(*val as f64),
            Self::Bool(val) => Ok((*val as i64) as f64),
            Self::String(val) => match val.parse::<f64>() {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {val} to f64")),
            },
            Self::RawData(_) => Err("Unable to convert raw data to f64".to_string()),
        }
    }

    pub fn as_f32(&self) -> Result<f32, String> {
        match self {
            Self::Double(val) => Ok(*val as f32),
            Self::Float(val) => Ok(*val),
            Self::Integer(val) => Ok(*val as f32),
            Self::Long(val) => Ok(*val as f32),
            Self::Bool(val) => Ok((*val as i32) as f32),
            Self::String(val) => match val.parse::<f32>() {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {val} to f32")),
            },
            Self::RawData(_) => Err("Unable to convert raw data to f32".to_string()),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Self::Double(val) => val.to_string(),
            Self::Float(val) => val.to_string(),
            Self::Integer(val) => val.to_string(),
            Self::Long(val) => val.to_string(),
            Self::Bool(val) => val.to_string(),
            Self::String(val) => val.clone(),
            Self::RawData(val) => String::from_utf8_lossy(val).to_string(),
        }
    }
}
