//! Functionality for parsing and using PICA200 shaders on the 3DS. This module
//! does not compile shaders, but enables using pre-compiled shaders at runtime.
//!
//! For more details about the PICA200 compiler / shader language, see
//! documentation for <https://github.com/devkitPro/picasso>.

use std::error::Error;
use std::ffi::CString;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Arc;

use crate::uniform;

/// A PICA200 shader program. It may have one or both of:
///
/// * A [vertex](Type::Vertex) shader [`Library`]
/// * A [geometry](Type::Geometry) shader [`Library`]
///
/// The PICA200 does not support user-programmable fragment shaders.
#[doc(alias = "shaderProgram_s")]
#[must_use]
#[derive(Clone)]
pub struct Program {
    program: ctru_sys::shaderProgram_s,
    /// needs to be pin'd to work properly with C3D_Context BindProgram
    _p: PhantomPinned,
}

unsafe impl Send for Program {}
unsafe impl Sync for Program {}

impl Program {
    /// Create a new shader program from a vertex shader.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * the shader program cannot be initialized
    /// * the input shader is not a vertex shader or is otherwise invalid
    #[doc(alias = "shaderProgramInit")]
    #[doc(alias = "shaderProgramSetVsh")]
    pub fn new(vertex_shader: Entrypoint) -> Result<Self, ctru::Error> {
        let mut program = unsafe {
            let mut program = MaybeUninit::uninit();
            let result = ctru_sys::shaderProgramInit(program.as_mut_ptr());
            if result != 0 {
                return Err(ctru::Error::from(result));
            }
            program.assume_init()
        };

        let ret = unsafe { ctru_sys::shaderProgramSetVsh(&mut program, vertex_shader.as_raw()) };

        if ret == 0 {
            Ok(Self {
                program,
                _p: PhantomPinned,
            })
        } else {
            Err(ctru::Error::from(ret))
        }
    }

    /// Set the geometry shader for a given program.
    ///
    /// # Errors
    ///
    /// Returns an error if the input shader is not a geometry shader or is
    /// otherwise invalid.
    #[doc(alias = "shaderProgramSetGsh")]
    pub fn set_geometry_shader(
        &mut self,
        geometry_shader: Entrypoint,
        stride: u8,
    ) -> Result<(), ctru::Error> {
        let ret = unsafe {
            ctru_sys::shaderProgramSetGsh(&mut self.program, geometry_shader.as_raw(), stride)
        };

        if ret == 0 {
            Ok(())
        } else {
            Err(ctru::Error::from(ret))
        }
    }

    /// Get the index of a uniform by name.
    ///
    /// # Errors
    ///
    /// * If the given `name` contains a null byte
    /// * If a uniform with the given `name` could not be found
    #[doc(alias = "shaderInstanceGetUniformLocation")]
    pub fn get_uniform(&self, name: &str) -> crate::Result<uniform::Index> {
        let vertex_instance = self.program.vertexShader;
        assert!(
            !vertex_instance.is_null(),
            "vertex shader should never be null!"
        );

        let name = CString::new(name)?;

        let idx =
            unsafe { ctru_sys::shaderInstanceGetUniformLocation(vertex_instance, name.as_ptr()) };

        if idx < 0 {
            Err(crate::Error::NotFound)
        } else {
            Ok((idx as u8).into())
        }
    }

    pub(crate) fn as_raw(self: &Pin<Arc<Self>>) -> *const ctru_sys::shaderProgram_s {
        &self.program
    }
}

static_assertions::assert_impl_all!(Program: Send, Sync);
static_assertions::assert_not_impl_any!(Program: Unpin);

impl Drop for Program {
    #[doc(alias = "shaderProgramFree")]
    fn drop(&mut self) {
        unsafe {
            let _ = ctru_sys::shaderProgramFree(&mut self.program);
        }
    }
}

/// The type of a shader.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Type {
    /// A vertex shader.
    Vertex = ctru_sys::GPU_VERTEX_SHADER,
    /// A geometry shader.
    Geometry = ctru_sys::GPU_GEOMETRY_SHADER,
}

impl From<Type> for u8 {
    fn from(value: Type) -> Self {
        value as u8
    }
}

/// A PICA200 Shader Library (commonly called DVLB). This can be comprised of
/// one or more [`Entrypoint`]s, but most commonly has one vertex shader and an
/// optional geometry shader.
///
/// This is the result of parsing a shader binary (`.shbin`), and the resulting
/// [`Entrypoint`]s can be used as part of a [`Program`].
#[doc(alias = "DVLB_s")]
#[derive(Debug)]
pub struct Library(NonNull<ctru_sys::DVLB_s>);

// Safety: we are the owner of the DVLB
unsafe impl Send for Library {}
unsafe impl Sync for Library {}

impl Library {
    /// Parse a new shader library from input bytes.
    ///
    /// # Errors
    ///
    /// An error is returned if the input data does not have an alignment of 4
    /// (cannot be safely converted to `&[u32]`).
    #[doc(alias = "DVLB_ParseFile")]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let aligned: &[u32] = bytemuck::try_cast_slice(bytes)?;
        let lib = unsafe {
            ctru_sys::DVLB_ParseFile(
                // SAFETY: we're trusting the parse implementation doesn't mutate
                // the contents of the data. From a quick read it looks like that's
                // correct and it should just take a const arg in the API.
                aligned.as_ptr().cast_mut(),
                aligned.len().try_into()?,
            )
        };
        let lib = NonNull::new(lib).ok_or(Box::new(super::Error::FailedToInitialize))?;
        Ok(Self(lib))
    }

    /// Get the number of [`Entrypoint`]s in this shader library.
    #[must_use]
    #[doc(alias = "numDVLE")]
    pub fn len(&self) -> usize {
        unsafe { self.0.as_ref().numDVLE as usize }
    }

    /// Whether the library has any [`Entrypoint`]s or not.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the [`Entrypoint`] at the given index, if present.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<Entrypoint> {
        if index < self.len() {
            Some(Entrypoint {
                ptr: unsafe { self.0.as_ref().DVLE.add(index) },
                _library: self,
            })
        } else {
            None
        }
    }
    pub fn get_uniform(&self, name: &str) -> Option<uniform::Index> {
        let dvle = unsafe { (*self.0.as_ptr()).DVLE.cast_const() };
        assert!(!dvle.is_null(), "dvle should not be null");
        let name = CString::new(name).ok()?;

        // Safety: This only reads from it, but its a C api so its not const
        let idx = unsafe { ctru_sys::DVLE_GetUniformRegister(dvle.cast_mut(), name.as_ptr()) };

        if idx < 0 {
            None
        } else {
            Some((idx as u8).into())
        }
    }

    fn as_raw(&mut self) -> *mut ctru_sys::DVLB_s {
        self.0.as_ptr()
    }
}

impl Drop for Library {
    #[doc(alias = "DVLB_Free")]
    fn drop(&mut self) {
        unsafe {
            ctru_sys::DVLB_Free(self.as_raw());
        }
    }
}

/// A shader library entrypoint (also called DVLE). This represents either a
/// vertex or a geometry shader.
#[derive(Clone, Copy)]
pub struct Entrypoint<'lib> {
    ptr: *mut ctru_sys::DVLE_s,
    _library: &'lib Library,
}

impl<'lib> Entrypoint<'lib> {
    fn as_raw(self) -> *mut ctru_sys::DVLE_s {
        self.ptr
    }
}
