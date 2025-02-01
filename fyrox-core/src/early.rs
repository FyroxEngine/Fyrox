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

#[macro_export]
macro_rules! some_or_return {
    ($expr:expr) => {{
        if let Some(v) = $expr {
            v
        } else {
            return;
        }
    }};
    ($expr:expr, $default:expr) => {{
        if let Some(v) = $expr {
            v
        } else {
            return $default;
        }
    }};
}

#[macro_export]
macro_rules! ok_or_return {
    ($expr:expr) => {{
        if let Ok(v) = $expr {
            v
        } else {
            return;
        }
    }};
    ($expr:expr, $default:expr) => {{
        if let Ok(v) = $expr {
            v
        } else {
            return $default;
        }
    }};
}

#[macro_export]
macro_rules! some_or_continue {
    ($expr:expr) => {{
        if let Some(v) = $expr {
            v
        } else {
            continue;
        }
    }};
    ($expr:expr, $lifetime:lifetime) => {{
        if let Some(v) = $expr {
            v
        } else {
            continue $lifetime;
        }
    }};
}

#[macro_export]
macro_rules! ok_or_continue {
    ($expr:expr) => {{
        if let Ok(v) = $expr {
            v
        } else {
            continue;
        }
    }};
    ($expr:expr, $lifetime:lifetime) => {{
        if let Ok(v) = $expr {
            v
        } else {
            continue $lifetime;
        }
    }};
}

#[macro_export]
macro_rules! some_or_break {
    ($expr:expr) => {{
        if let Some(v) = $expr {
            v
        } else {
            break;
        }
    }};
    ($expr:expr, $lifetime:lifetime) => {{
        if let Some(v) = $expr {
            v
        } else {
            break $lifetime;
        }
    }};
}

#[macro_export]
macro_rules! ok_or_break {
    ($expr:expr) => {{
        if let Ok(v) = $expr {
            v
        } else {
            break;
        }
    }};
    ($expr:expr, $lifetime:lifetime) => {{
        if let Ok(v) = $expr {
            v
        } else {
            break $lifetime;
        }
    }};
}
