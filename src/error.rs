use wasm_bindgen::JsError;

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self { message: value }
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<Error> for JsError {
    fn from(value: Error) -> Self {
        JsError::new(&value.message)
    }
}
