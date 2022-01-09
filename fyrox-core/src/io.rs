use std::io::Error;
use std::path::Path;

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
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        path.as_ref().exists()
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
