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
            FbxAttribute::Double(double) => write!(f, "{double}"),
            FbxAttribute::Float(float) => write!(f, "{float}"),
            FbxAttribute::Integer(integer) => write!(f, "{integer}"),
            FbxAttribute::Long(long) => write!(f, "{long}"),
            FbxAttribute::Bool(boolean) => write!(f, "{boolean}"),
            FbxAttribute::String(string) => write!(f, "{string}"),
            FbxAttribute::RawData(raw) => write!(f, "{raw:?}"),
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
            FbxAttribute::String(val) => match val.parse::<i32>() {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {val} to i32")),
            },
            FbxAttribute::RawData(_) => Err("Unable to convert raw data to i32".to_string()),
        }
    }

    pub fn as_i64(&self) -> Result<i64, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val as i64),
            FbxAttribute::Float(val) => Ok(*val as i64),
            FbxAttribute::Integer(val) => Ok(i64::from(*val)),
            FbxAttribute::Long(val) => Ok(*val),
            FbxAttribute::Bool(val) => Ok(*val as i64),
            FbxAttribute::String(val) => match val.parse::<i64>() {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {val} to i64")),
            },
            FbxAttribute::RawData(_) => Err("Unable to convert raw data to i64".to_string()),
        }
    }

    pub fn as_f64(&self) -> Result<f64, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val),
            FbxAttribute::Float(val) => Ok(f64::from(*val)),
            FbxAttribute::Integer(val) => Ok(f64::from(*val)),
            FbxAttribute::Long(val) => Ok(*val as f64),
            FbxAttribute::Bool(val) => Ok((*val as i64) as f64),
            FbxAttribute::String(val) => match val.parse::<f64>() {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {val} to f64")),
            },
            FbxAttribute::RawData(_) => Err("Unable to convert raw data to f64".to_string()),
        }
    }

    pub fn as_f32(&self) -> Result<f32, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val as f32),
            FbxAttribute::Float(val) => Ok(*val),
            FbxAttribute::Integer(val) => Ok(*val as f32),
            FbxAttribute::Long(val) => Ok(*val as f32),
            FbxAttribute::Bool(val) => Ok((*val as i32) as f32),
            FbxAttribute::String(val) => match val.parse::<f32>() {
                Ok(i) => Ok(i),
                Err(_) => Err(format!("Unable to convert string {val} to f32")),
            },
            FbxAttribute::RawData(_) => Err("Unable to convert raw data to f32".to_string()),
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
            FbxAttribute::RawData(val) => String::from_utf8_lossy(val).to_string(),
        }
    }
}
