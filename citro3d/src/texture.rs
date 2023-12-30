use std::ptr::NonNull;

/// Format of the texture bytes
#[doc(alias = "GPU_TEXCOLOR")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TexFormat {
    /// 8-bit Red + 8-bit Green + 8-bit Blue + 8-bit Alpha
    Rgba8 = ctru_sys::GPU_RGBA8,
    /// 8-bit Red + 8-bit Green + 8-bit Blue
    Rgb8 = ctru_sys::GPU_RGB8,
    /// 5-bit Red + 5-bit Green + 5-bit Blue + 1-bit Alpha
    Rgba5551 = ctru_sys::GPU_RGBA5551,
    /// 5-bit Red + 6-bit Green + 5-bit Blue
    Rgb565 = ctru_sys::GPU_RGB565,
    /// 4-bit Red + 4-bit Green + 4-bit Blue + 4-bit Alpha
    Rgba4 = ctru_sys::GPU_RGBA4,
    /// 8-bit Luminance + 8-bit Alpha
    La8 = ctru_sys::GPU_LA8,
    /// 8-bit Hi + 8-bit Lo
    HiLo8 = ctru_sys::GPU_HILO8,
    /// 8-bit Luminance
    L8 = ctru_sys::GPU_L8,
    /// 8-bit Alpha
    A8 = ctru_sys::GPU_A8,
    /// 4-bit Luminance + 4-bit Alpha
    La4 = ctru_sys::GPU_LA4,
    /// 4-bit Luminance
    L4 = ctru_sys::GPU_L4,
    /// 4-bit Alpha
    A4 = ctru_sys::GPU_A4,
    /// ETC1 texture compression
    Etc1 = ctru_sys::GPU_ETC1,
    /// ETC1 texture compression + 4-bit Alpha
    Etc1A4 = ctru_sys::GPU_ETC1A4,
}

impl TexFormat {
    /// Bits needed to store each pixel
    pub fn bits_per_pixel(&self) -> usize {
        match self {
            TexFormat::Rgba8 => 32,
            TexFormat::Rgb8 => 24,
            TexFormat::Rgba5551
            | TexFormat::Rgb565
            | TexFormat::Rgba4
            | TexFormat::La8
            | TexFormat::HiLo8 => 16,
            TexFormat::L8 | TexFormat::A8 | TexFormat::La4 | TexFormat::Etc1A4 => 8,
            TexFormat::L4 | TexFormat::A4 | TexFormat::Etc1 => 4,
        }
    }
}

impl TryFrom<ctru_sys::GPU_TEXCOLOR> for TexFormat {
    type Error = super::Error;

    fn try_from(value: ctru_sys::GPU_TEXCOLOR) -> Result<Self, Self::Error> {
        match value {
            ctru_sys::GPU_RGBA8 => Ok(Self::Rgba8),
            ctru_sys::GPU_RGB8 => Ok(Self::Rgb8),
            ctru_sys::GPU_RGBA5551 => Ok(Self::Rgba5551),
            ctru_sys::GPU_RGB565 => Ok(Self::Rgb565),
            ctru_sys::GPU_RGBA4 => Ok(Self::Rgba4),
            ctru_sys::GPU_LA8 => Ok(Self::La8),
            ctru_sys::GPU_HILO8 => Ok(Self::HiLo8),
            ctru_sys::GPU_L8 => Ok(Self::L8),
            ctru_sys::GPU_A8 => Ok(Self::A8),
            ctru_sys::GPU_LA4 => Ok(Self::La4),
            ctru_sys::GPU_L4 => Ok(Self::L4),
            ctru_sys::GPU_A4 => Ok(Self::A4),
            ctru_sys::GPU_ETC1 => Ok(Self::Etc1),
            ctru_sys::GPU_ETC1A4 => Ok(Self::Etc1A4),
            _ => Err(super::Error::NotFound),
        }
    }
}

#[doc(alias = "GPU_TEXTURE_FILTER_PARAM")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureFilterParam {
    /// Nearest-neighbor interpolation.
    Nearest = ctru_sys::GPU_NEAREST,
    /// Linear interpolation.
    Linear = ctru_sys::GPU_LINEAR,
}

#[doc(alias = "GPU_TEXTURE_WRAP_PARAM")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureWrapParam {
    /// Clamps to edge.
    ClampToEdge = ctru_sys::GPU_CLAMP_TO_EDGE,
    /// Clamps to border.
    ClampToBorder = ctru_sys::GPU_CLAMP_TO_BORDER,
    /// Repeats texture.
    Repeat = ctru_sys::GPU_REPEAT,
    /// Repeats with mirrored texture.
    MirroredRepeat = ctru_sys::GPU_MIRRORED_REPEAT,
}

pub struct Tex(NonNull<citro3d_sys::C3D_Tex>);

impl Tex {
    #[doc(alias = "C3D_TexInit")]
    pub fn new(width: u16, height: u16, format: TexFormat) -> super::Result<Self> {
        let raw = unsafe {
            let mut raw = Box::<citro3d_sys::C3D_Tex>::new_uninit();
            if !citro3d_sys::C3D_TexInit(raw.as_mut_ptr(), width, height, format as _) {
                return Err(super::Error::FailedToInitialize);
            }
            raw.assume_init()
        };
        let raw = NonNull::new(Box::into_raw(raw)).ok_or(super::Error::FailedToInitialize)?;
        Ok(Self(raw))
    }
    pub fn width(&self) -> u16 {
        unsafe { self.0.as_ref().__bindgen_anon_2.__bindgen_anon_1.width }
    }
    pub fn height(&self) -> u16 {
        unsafe { self.0.as_ref().__bindgen_anon_2.__bindgen_anon_1.height }
    }

    #[doc(alias = "C3D_TexBind")]
    pub fn bind(&self, unit_id: i32) {
        unsafe { citro3d_sys::C3D_TexBind(unit_id, self.as_raw().cast_mut()) }
    }
    pub fn format(&self) -> TexFormat {
        TexFormat::try_from(unsafe { self.0.as_ref().fmt() }).expect("unknown texture colour type")
    }

    #[doc(alias = "C3D_TexUpload")]
    pub fn upload<T: AsRef<[u8]>>(&self, data: T) {
        let buf = data.as_ref();

        let (width, height) = (self.width(), self.height());
        let (width, height) = (width as usize, height as usize);
        assert!(buf.len() >= width * height * self.format().bits_per_pixel() / 8);

        unsafe { citro3d_sys::C3D_TexUpload(self.as_raw().cast_mut(), buf.as_ptr().cast()) }
    }

    #[doc(alias = "C3D_TexSetFilter")]
    pub fn set_filter(&self, mag_filter: TextureFilterParam, min_filter: TextureFilterParam) {
        unsafe {
            citro3d_sys::C3D_TexSetFilter(
                self.as_raw().cast_mut(),
                mag_filter as u32,
                min_filter as u32,
            )
        }
    }

    #[doc(alias = "C3D_TexSetWrap")]
    pub fn set_wrap(&self, wrap_s: TextureWrapParam, wrap_t: TextureWrapParam) {
        unsafe {
            citro3d_sys::C3D_TexSetWrap(self.as_raw().cast_mut(), wrap_s as u32, wrap_t as u32)
        }
    }

    pub fn as_raw(&self) -> *const citro3d_sys::C3D_Tex {
        self.0.as_ptr() as *const _
    }

    pub fn as_raw_mut(&mut self) -> *mut citro3d_sys::C3D_Tex {
        self.0.as_ptr()
    }
}

impl Drop for Tex {
    #[doc(alias = "C3D_TexDelete")]
    fn drop(&mut self) {
        unsafe { citro3d_sys::C3D_TexDelete(self.as_raw().cast_mut()) }
    }
}
