//! Error type shared by every Rustille entry point.

use core::fmt;

/// Convenience alias for results produced by this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Everything that can go wrong while rendering.
///
/// The variants are deliberately coarse and stable: bindings map them onto
/// numeric codes (see the C ABI's `RustilleStatus`), so adding a variant is a
/// breaking change for them. The enum is `#[non_exhaustive]` so that adding one
/// stays possible without breaking Rust callers.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The image bytes could not be decoded.
    #[error("failed to decode image: {0}")]
    Decode(String),

    /// The container format was recognised but is not compiled in (or is not
    /// supported at all).
    #[error("unsupported image format: {0}")]
    UnsupportedFormat(String),

    /// A width or height was zero, or the pixel count would overflow.
    #[error("invalid dimensions: {0}")]
    InvalidDimensions(String),

    /// A raw pixel buffer did not match `width * height * bytes_per_pixel`.
    #[error("invalid buffer length: expected {expected} bytes, got {actual}")]
    InvalidBufferLength {
        /// Number of bytes implied by the declared dimensions.
        expected: usize,
        /// Number of bytes actually provided.
        actual: usize,
    },

    /// A [`RenderOptions`](crate::RenderOptions) field was out of range.
    #[error("invalid option: {0}")]
    InvalidOptions(String),

    /// Reading or writing failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// A short, stable, machine-readable tag for the error.
    ///
    /// Bindings use this when they need a string discriminant instead of a
    /// numeric one (Python exceptions, JS error names, …).
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Error::Decode(_) => ErrorKind::Decode,
            Error::UnsupportedFormat(_) => ErrorKind::UnsupportedFormat,
            Error::InvalidDimensions(_) => ErrorKind::InvalidDimensions,
            Error::InvalidBufferLength { .. } => ErrorKind::InvalidBufferLength,
            Error::InvalidOptions(_) => ErrorKind::InvalidOptions,
            Error::Io(_) => ErrorKind::Io,
        }
    }

    pub(crate) fn invalid_dimensions(msg: impl Into<String>) -> Self {
        Error::InvalidDimensions(msg.into())
    }

    pub(crate) fn invalid_options(msg: impl Into<String>) -> Self {
        Error::InvalidOptions(msg.into())
    }
}

/// Discriminant of [`Error`], without the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// See [`Error::Decode`].
    Decode,
    /// See [`Error::UnsupportedFormat`].
    UnsupportedFormat,
    /// See [`Error::InvalidDimensions`].
    InvalidDimensions,
    /// See [`Error::InvalidBufferLength`].
    InvalidBufferLength,
    /// See [`Error::InvalidOptions`].
    InvalidOptions,
    /// See [`Error::Io`].
    Io,
}

impl ErrorKind {
    /// The tag as a lowercase, hyphen-free identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorKind::Decode => "decode",
            ErrorKind::UnsupportedFormat => "unsupported_format",
            ErrorKind::InvalidDimensions => "invalid_dimensions",
            ErrorKind::InvalidBufferLength => "invalid_buffer_length",
            ErrorKind::InvalidOptions => "invalid_options",
            ErrorKind::Io => "io",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "_image")]
impl From<image::ImageError> for Error {
    fn from(err: image::ImageError) -> Self {
        use image::ImageError;
        match err {
            ImageError::IoError(io) => Error::Io(io),
            ImageError::Unsupported(u) => Error::UnsupportedFormat(u.to_string()),
            other => Error::Decode(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip_through_display() {
        let err = Error::InvalidBufferLength {
            expected: 16,
            actual: 3,
        };
        assert_eq!(err.kind(), ErrorKind::InvalidBufferLength);
        assert_eq!(err.kind().to_string(), "invalid_buffer_length");
        assert!(err.to_string().contains("expected 16"));
    }

    #[test]
    fn io_errors_convert() {
        let err: Error = std::io::Error::other("nope").into();
        assert_eq!(err.kind(), ErrorKind::Io);
    }
}
