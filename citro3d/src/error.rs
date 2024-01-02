//! General-purpose error and result types returned by public APIs of this crate.

use std::ffi::NulError;
use std::num::TryFromIntError;
use std::sync::TryLockError;

/// The common result type returned by `citro3d` functions.
pub type Result<T> = std::result::Result<T, Error>;

// TODO probably want a similar type to ctru::Result to make it easier to convert
// nonzero result codes to errors.

/// The common error type that may be returned by `citro3d` functions.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// C3D error code.
    #[error("C3D error, code: {0}")]
    System(libc::c_int),
    /// A C3D object or context could not be initialized.
    #[error("a C3D object or context failed to initialize")]
    FailedToInitialize,
    /// A size parameter was specified that cannot be converted to the proper type.
    #[error("specified size parameter is invalid")]
    InvalidSize,
    /// Failed to select the given render target for drawing to.
    #[error("render target is invalid")]
    InvalidRenderTarget,
    /// Indicates that a reference could not be obtained because a lock is already
    /// held on the requested object.
    #[error("a lock is already held for requested object")]
    LockHeld,
    /// Indicates that too many vertex attributes were registered (max 12 supported).
    #[error("too many vertex attribute requested (max of 12)")]
    TooManyAttributes,
    /// Indicates that too many vertex buffer objects were registered (max 12 supported).
    #[error("too many vertex buffer objects registered (max of 12)")]
    TooManyBuffers,
    /// The given memory could not be converted to a physical address for sharing
    /// with the GPU. Data should be allocated with [`ctru::linear`].
    #[error("invalid memory location, address should be allocated with `ctru::linear`")]
    InvalidMemoryLocation,
    /// The given name was not valid for the requested purpose.
    #[error("provided name is invalid")]
    InvalidName,
    /// The requested resource could not be found.
    #[error("requested resource could not be found")]
    NotFound,
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Self {
        Self::InvalidSize
    }
}

impl<T> From<TryLockError<T>> for Error {
    fn from(_: TryLockError<T>) -> Self {
        Self::LockHeld
    }
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Self::InvalidName
    }
}
