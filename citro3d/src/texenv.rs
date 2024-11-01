//! Texture combiner support. See <https://www.khronos.org/opengl/wiki/Texture_Combiners>
//! for more details.

use std::ptr::NonNull;

use bitflags::bitflags;

/// A texture combiner, also called a "texture environment" (hence the struct name).
/// See also [`texenv.h` documentation](https://oreo639.github.io/citro3d/texenv_8h.html).
#[doc(alias = "C3D_TexEnv")]
#[derive(Debug)]
pub struct TexEnv(NonNull<citro3d_sys::C3D_TexEnv>);

/// We don't own the pointer but we are the only access safe code will have to it
/// and there is only 1 of us for each pointer in that case
unsafe impl Send for TexEnv {}
unsafe impl Sync for TexEnv {}

// https://oreo639.github.io/citro3d/texenv_8h.html#a9eda91f8e7252c91f873b1d43e3728b6
pub(crate) const TEXENV_COUNT: usize = 6;

impl TexEnv {
    pub(crate) fn new(stage: Stage) -> Self {
        let mut result = unsafe {
            Self(
                NonNull::new(citro3d_sys::C3D_GetTexEnv(stage.0 as _))
                    .expect("failed to get textenv, this is a bug"),
            )
        };
        result.reset();
        result
    }

    /// Re-initialize the texture combiner to its default state.
    pub fn reset(&mut self) {
        unsafe {
            citro3d_sys::C3D_TexEnvInit(self.0.as_ptr());
        }
    }

    /// Configure the source values of the texture combiner.
    ///
    /// # Parameters
    ///
    /// - `mode`: which [`Mode`]\(s) to set the sourc operand(s) for.
    /// - `source0`: the first [`Source`] operand to the texture combiner
    /// - `source1` and `source2`: optional additional [`Source`] operands to use
    #[doc(alias = "C3D_TexEnvSrc")]
    pub fn src(
        &mut self,
        mode: Mode,
        source0: Source,
        source1: Option<Source>,
        source2: Option<Source>,
    ) -> &mut Self {
        unsafe {
            citro3d_sys::C3D_TexEnvSrc(
                self.0.as_ptr(),
                mode.bits(),
                source0 as _,
                source1.unwrap_or(Source::PrimaryColor) as _,
                source2.unwrap_or(Source::PrimaryColor) as _,
            );
        }
        self
    }

    /// Configure the texture combination function.
    ///
    /// # Parameters
    ///
    /// - `mode`: the [`Mode`]\(s) the combination function will apply to.
    /// - `func`: the [`CombineFunc`] used to combine textures.
    #[doc(alias = "C3D_TexEnvFunc")]
    pub fn func(&mut self, mode: Mode, func: CombineFunc) -> &mut Self {
        unsafe {
            citro3d_sys::C3D_TexEnvFunc(self.0.as_ptr(), mode.bits(), func as _);
        }

        self
    }
}

bitflags! {
    /// Whether to operate on colors, alpha values, or both.
    #[doc(alias = "C3D_TexEnvMode")]
    pub struct Mode: citro3d_sys::C3D_TexEnvMode {
        #[allow(missing_docs)]
        const RGB = citro3d_sys::C3D_RGB;
        #[allow(missing_docs)]
        const ALPHA = citro3d_sys::C3D_Alpha;
        #[allow(missing_docs)]
        const BOTH = citro3d_sys::C3D_Both;
    }
}

/// A source operand of a [`TexEnv`]'s texture combination.
#[doc(alias = "GPU_TEVSRC")]
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[non_exhaustive]
pub enum Source {
    PrimaryColor = ctru_sys::GPU_PRIMARY_COLOR,
    FragmentPrimaryColor = ctru_sys::GPU_FRAGMENT_PRIMARY_COLOR,
    FragmentSecondaryColor = ctru_sys::GPU_FRAGMENT_SECONDARY_COLOR,
    Texture0 = ctru_sys::GPU_TEXTURE0,
    Texture1 = ctru_sys::GPU_TEXTURE1,
    Texture2 = ctru_sys::GPU_TEXTURE2,
    Texture3 = ctru_sys::GPU_TEXTURE3,
    PreviousBuffer = ctru_sys::GPU_PREVIOUS_BUFFER,
    Constant = ctru_sys::GPU_CONSTANT,
    Previous = ctru_sys::GPU_PREVIOUS,
}

/// The combination function to apply to the [`TexEnv`] operands.
#[doc(alias = "GPU_COMBINEFUNC")]
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[non_exhaustive]
pub enum CombineFunc {
    Replace = ctru_sys::GPU_REPLACE,
    Modulate = ctru_sys::GPU_MODULATE,
    Add = ctru_sys::GPU_ADD,
    AddSigned = ctru_sys::GPU_ADD_SIGNED,
    Interpolate = ctru_sys::GPU_INTERPOLATE,
    Subtract = ctru_sys::GPU_SUBTRACT,
    Dot3Rgb = ctru_sys::GPU_DOT3_RGB,
    // Added in libcrtu 2.3.0:
    // Dot3Rgba = ctru_sys::GPU_DOT3_RGBA,
}

/// A texture combination stage identifier. This index doubles as the order
/// in which texture combinations will be applied.
// (I think?)
#[derive(Copy, Clone, Debug)]
pub struct Stage(pub(crate) usize);

impl Stage {
    /// Get a stage index. Valid indices range from 0 to 5.
    pub fn new(index: usize) -> Option<Self> {
        (index < 6).then_some(Self(index))
    }
}
