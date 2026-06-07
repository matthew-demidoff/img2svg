use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("pixel buffer length {len} does not match {width}x{height} RGBA")]
    Dimensions { width: u32, height: u32, len: usize },

    #[error("image has zero width or height")]
    EmptyImage,

    #[error("tracer failed: {0}")]
    Trace(String),
}
