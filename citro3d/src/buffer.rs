//! Configure vertex buffer objects to be sent to the GPU for rendering.
//!
//! See the [`attrib`] module for details on how to describe the shape and type
//! of the VBO data.

use std::mem::MaybeUninit;

use crate::attrib;

/// Vertex buffer info. This struct is used to describe the shape of the buffer
/// data to be sent to the GPU for rendering.
#[derive(Debug, Clone, Copy)]
#[doc(alias = "C3D_BufInfo")]
pub struct Info(pub(crate) citro3d_sys::C3D_BufInfo);

/// A slice of buffer data. This borrows the buffer data and can be thought of
/// as similar to `&[T]` obtained by slicing a `Vec<T>`.
#[derive(Debug, Clone, Copy)]
pub struct Slice<'buf> {
    index: libc::c_int,
    size: libc::c_int,
    buf_info: &'buf Info,
    // TODO: should we encapsulate the primitive here too, and require it when the
    // slice is registered? Could there ever be a use case to draw different primitives
    // using the same backing data???
}

impl Slice<'_> {
    /// Get the index into the buffer for this slice.
    pub fn index(&self) -> libc::c_int {
        self.index
    }

    /// Get the length of the slice.
    #[must_use]
    pub fn len(&self) -> libc::c_int {
        self.size
    }

    /// Return whether or not the slice has any elements.
    pub fn is_empty(&self) -> bool {
        self.len() <= 0
    }

    /// Get the buffer info this slice is associated with.
    pub fn info(&self) -> &Info {
        self.buf_info
    }
}

/// The geometric primitive to draw (i.e. what shapes the buffer data describes).
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
#[doc(alias = "GPU_Primitive_t")]
pub enum Primitive {
    /// Draw triangles (3 vertices per triangle).
    Triangles = ctru_sys::GPU_TRIANGLES,
    /// Draw a triangle strip (each vertex shared by 1-3 triangles).
    TriangleStrip = ctru_sys::GPU_TRIANGLE_STRIP,
    /// Draw a triangle fan (first vertex shared by all triangles).
    TriangleFan = ctru_sys::GPU_TRIANGLE_FAN,
    /// Geometry primitive. Can be used for more complex use cases like geometry
    /// shaders that output custom primitives.
    GeometryPrim = ctru_sys::GPU_GEOMETRY_PRIM,
}

impl Default for Info {
    #[doc(alias = "BufInfo_Init")]
    fn default() -> Self {
        let mut info = MaybeUninit::zeroed();
        let info = unsafe {
            citro3d_sys::BufInfo_Init(info.as_mut_ptr());
            info.assume_init()
        };
        Self(info)
    }
}

impl Info {
    /// Construct buffer info without any registered data.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn copy_from(raw: *const citro3d_sys::C3D_BufInfo) -> Option<Self> {
        if raw.is_null() {
            None
        } else {
            // This is less efficient than returning a pointer or something, but it's
            // safer since we don't know the lifetime of the pointee
            Some(Self(unsafe { *raw }))
        }
    }

    /// Register vertex buffer object data. The resulting [`Slice`] will have its
    /// lifetime tied to both this [`Info`] and the passed-in VBO. `vbo_data` is
    /// assumed to use one `T` per drawn primitive, and its layout is assumed to
    /// match the given `attrib_info`
    ///
    /// # Errors
    ///
    /// Registering VBO data may fail:
    ///
    /// * if `vbo_data` is not allocated with the [`ctru::linear`] allocator
    /// * if the maximum number (12) of VBOs are already registered
    #[doc(alias = "BufInfo_Add")]
    pub fn add<'this, 'vbo, 'idx, T>(
        &'this mut self,
        vbo_data: &'vbo [T],
        attrib_info: &attrib::Info,
    ) -> crate::Result<Slice<'idx>>
    where
        'this: 'idx,
        'vbo: 'idx,
    {
        unsafe {
            self.add_bytes(
                std::slice::from_raw_parts(
                    vbo_data.as_ptr().cast(),
                    std::mem::size_of_val(vbo_data),
                ),
                attrib_info,
                std::mem::size_of::<T>() as u32,
            )
        }
    }

    /// Add vbo bytes directly
    ///
    /// This is the same as [`Info::add`] except it requires manually specifying the
    /// stride for each set of attributes, this is useful if you don't know the size
    /// at compile time
    ///
    /// # Safety
    /// `vbo_data` must have data matching `attrib_info` every `stride` bytes or strangeness
    /// will occur
    #[doc(alias = "BufInfo_Add")]
    pub unsafe fn add_bytes<'this, 'vbo, 'idx>(
        &'this mut self,
        vbo_data: &'vbo [u8],
        attrib_info: &attrib::Info,
        stride: u32,
    ) -> crate::Result<Slice<'idx>>
    where
        'this: 'idx,
        'vbo: 'idx,
    {
        // SAFETY: the lifetime of the VBO data is encapsulated in the return value's
        // 'vbo lifetime, and the pointer to &mut self.0 is used to access values
        // in the BufInfo, not copied to be used later.
        let res = unsafe {
            citro3d_sys::BufInfo_Add(
                &mut self.0,
                vbo_data.as_ptr().cast(),
                stride as isize,
                attrib_info.attr_count(),
                attrib_info.permutation(),
            )
        };

        // Error codes from <https://github.com/devkitPro/citro3d/blob/master/source/buffers.c#L11>
        match res {
            ..=-3 => Err(crate::Error::System(res)),
            -2 => Err(crate::Error::InvalidMemoryLocation),
            -1 => Err(crate::Error::TooManyBuffers),
            _ => Ok(Slice {
                index: res,
                size: vbo_data.len().try_into()?,
                buf_info: self,
            }),
        }
    }
}
