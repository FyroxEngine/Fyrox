use std::{io::Error, path::Path};

#[derive(Debug)]
pub enum FileLoadError {
    Io(std::io::Error),
    Custom(String),
}

impl From<std::io::Error> for FileLoadError {
    fn from(e: Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(target_os = "android")]
pub static ANDROID_APP: once_cell::sync::OnceCell<android_activity::AndroidApp> =
    once_cell::sync::OnceCell::new();

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for FileLoadError {
    fn from(value: wasm_bindgen::JsValue) -> Self {
        let string = match js_sys::JSON::stringify(&value) {
            Ok(string) => String::from(string),
            Err(_) => format!("{:?}", value),
        };
        Self::Custom(string)
    }
}

pub async fn load_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, FileLoadError> {
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    #[cfg(target_os = "android")]
    {
        let asset_manager = ANDROID_APP
            .get()
            .ok_or_else(|| FileLoadError::Custom("ANDROID_APP is not set".to_string()))?
            .asset_manager();
        let mut opened_asset = asset_manager
            .open(&std::ffi::CString::new(path.as_ref().to_str().unwrap()).unwrap())
            .ok_or_else(|| FileLoadError::Custom(format!("File {:?} not found!", path.as_ref())))?;
        let bytes = opened_asset.buffer()?;
        Ok(bytes.to_vec())
    }

    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Uint8Array;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;

        match web_sys::window() {
            Some(window) => {
                let resp_value =
                    JsFuture::from(window.fetch_with_str(path.as_ref().to_str().unwrap())).await?;

                let resp: web_sys::Response = resp_value.dyn_into().unwrap();
                let data = JsFuture::from(resp.array_buffer().unwrap()).await?;
                let bytes = Uint8Array::new(&data).to_vec();
                Ok(bytes)
            }
            None => Err(FileLoadError::Custom("Window not found!".to_owned())),
        }
    }
}

pub async fn exists<P: AsRef<Path>>(path: P) -> bool {
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        path.as_ref().exists()
    }

    #[cfg(target_os = "android")]
    {
        ANDROID_APP
            .get()
            .map(|v| {
                v.asset_manager()
                    .open(&std::ffi::CString::new(path.as_ref().to_str().unwrap()).unwrap())
                    .is_some()
            })
            .unwrap_or_default()
    }

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;

        match web_sys::window() {
            Some(window) => {
                if let Ok(resp_value) =
                    JsFuture::from(window.fetch_with_str(path.as_ref().to_str().unwrap())).await
                {
                    let resp: web_sys::Response = resp_value.dyn_into().unwrap();

                    resp.status() == 200
                } else {
                    false
                }
            }
            None => false,
        }
    }
}

pub async fn is_dir<P: AsRef<Path>>(#[allow(unused)] path: P) -> bool {
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        path.as_ref().is_dir()
    }

    #[cfg(target_os = "android")]
    {
        ANDROID_APP
            .get()
            .map(|v| {
                v.asset_manager()
                    .open_dir(&std::ffi::CString::new(path.as_ref().to_str().unwrap()).unwrap())
                    .is_some()
            })
            .unwrap_or_default()
    }

    // TODO: Is directory checking possible on wasm?
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
}

pub async fn is_file<P: AsRef<Path>>(path: P) -> bool {
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        path.as_ref().is_file()
    }

    // On android and wasm the default exists logic works for files
    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    {
        exists(path).await
    }
}
