//! Bindings for accessing the lighting part of the GPU pipeline
//!
//! What does anything in this module mean? inspect this diagram: https://raw.githubusercontent.com/wwylele/misc-3ds-diagram/master/pica-pipeline.svg

use std::{marker::PhantomPinned, mem::MaybeUninit, pin::Pin};

use pin_array::PinArray;

use crate::{
    material::Material,
    math::{FVec3, FVec4},
};

/// Index for one of the 8 hardware lights in the GPU pipeline
///
/// Usually you don't want to construct one of these directly but use [`LightEnv::create_light`]
// Note we use a u8 here since usize is overkill and it saves a few bytes
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightIndex(u8);

const NB_LIGHTS: usize = 8;

impl LightIndex {
    /// Manually create a `LightIndex` with a specific index
    ///
    /// # Panics
    /// if `idx` out of range for the number of lights (>=8)
    pub fn new(idx: usize) -> Self {
        assert!(idx < NB_LIGHTS);
        Self(idx as u8)
    }
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Default)]
struct LightLutStorage {
    spot: Option<LutData>,
    diffuse_atten: Option<LutData>,
    _pin: PhantomPinned,
}

#[derive(Default)]
struct LightEnvStorage {
    lights: [Option<Light>; NB_LIGHTS],
    luts: [Option<LutData>; 6],
    _pin: PhantomPinned,
}

impl LightEnvStorage {
    fn lights_mut(self: Pin<&mut Self>) -> Pin<&mut [Option<Light>; NB_LIGHTS]> {
        unsafe { Pin::map_unchecked_mut(self, |s| &mut s.lights) }
    }
}

pub struct LightEnv {
    raw: citro3d_sys::C3D_LightEnv,
    /// The actual light data pointed to by the lights element of `raw`
    ///
    /// Note this is `Pin` as well as `Box` as `raw` means we are _actually_ self-referential which
    /// is horrible but the best bad option in this case
    lights: LightArray,
    luts: [Option<LutData>; 6],
    _pin: PhantomPinned,
}

pub struct Light {
    raw: citro3d_sys::C3D_Light,
    spot: Option<LutData>,
    diffuse_atten: Option<LutData>,
    _pin: PhantomPinned,
}

impl Default for LightEnv {
    fn default() -> Self {
        let raw = unsafe {
            let mut env = MaybeUninit::zeroed();
            citro3d_sys::C3D_LightEnvInit(env.as_mut_ptr());
            env.assume_init()
        };
        Self {
            raw,
            lights: Default::default(),
            luts: Default::default(),
            _pin: Default::default(),
        }
    }
}
impl LightEnv {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_material(self: Pin<&mut Self>, mat: Material) {
        let raw = mat.to_raw();
        // Safety: This takes a pointer but it actually memcpy's it so this doesn't dangle
        unsafe {
            citro3d_sys::C3D_LightEnvMaterial(self.as_raw_mut() as *mut _, (&raw) as *const _);
        }
    }

    pub fn lights(&self) -> &LightArray {
        &self.lights
    }

    pub fn lights_mut(self: Pin<&mut Self>) -> Pin<&mut LightArray> {
        unsafe { self.map_unchecked_mut(|s| &mut s.lights) }
    }

    pub fn light_mut(self: Pin<&mut Self>, idx: LightIndex) -> Option<Pin<&mut Light>> {
        self.lights_mut()
            .get_pin(idx.0 as usize)
            .unwrap()
            .as_pin_mut()
    }
    pub fn create_light(mut self: Pin<&mut Self>) -> Option<LightIndex> {
        let idx = self
            .lights()
            .iter()
            .enumerate()
            .find(|(_, n)| n.is_none())
            .map(|(n, _)| n)?;

        self.as_mut()
            .lights_mut()
            .get_pin(idx)
            .unwrap()
            .set(Some(Light::new(unsafe {
                MaybeUninit::zeroed().assume_init()
            })));

        let target = unsafe {
            self.as_mut()
                .lights_mut()
                .get_pin(idx)
                .unwrap()
                .map_unchecked_mut(|p| p.as_mut().unwrap())
        };
        let r =
            unsafe { citro3d_sys::C3D_LightInit(target.as_raw_mut(), self.as_raw_mut() as *mut _) };
        assert!(r >= 0, "C3D_LightInit should only fail if there are no free light slots but we checked that already, how did this happen?");
        assert_eq!(
            r as usize, idx,
            "citro3d chose a different light to us? this shouldn't be possible"
        );
        Some(LightIndex::new(idx))
    }
    ///
    pub fn connect_lut(mut self: Pin<&mut Self>, id: LightLutId, input: LutInput, data: LutData) {
        let idx = match id {
            LightLutId::D0 => Some(0),
            LightLutId::D1 => Some(1),
            LightLutId::SpotLightAttenuation => None,
            LightLutId::Fresnel => Some(2),
            LightLutId::ReflectBlue => Some(3),
            LightLutId::ReflectGreen => Some(4),
            LightLutId::ReflectRed => Some(5),
            LightLutId::DistanceAttenuation => None,
        };
        let (raw, lut) = unsafe {
            // this is needed to do structural borrowing as otherwise
            // the compiler rejects the reborrow needed with the pin
            let me = self.as_mut().get_unchecked_mut();
            let lut = idx.map(|i| me.luts[i].insert(data));
            let raw = &mut me.raw;
            let lut = match lut {
                Some(l) => (&mut l.0) as *mut _,
                None => core::ptr::null_mut(),
            };
            (raw, lut)
        };
        unsafe {
            citro3d_sys::C3D_LightEnvLut(raw, id as u32, input as u32, false, lut);
        }
    }

    pub fn as_raw(&self) -> &citro3d_sys::C3D_LightEnv {
        &self.raw
    }

    pub fn as_raw_mut(self: Pin<&mut Self>) -> &mut citro3d_sys::C3D_LightEnv {
        unsafe { &mut self.get_unchecked_mut().raw }
    }
}

impl Light {
    fn new(raw: citro3d_sys::C3D_Light) -> Self {
        Self {
            raw,
            spot: Default::default(),
            diffuse_atten: Default::default(),
            _pin: Default::default(),
        }
    }

    fn from_raw_ref(l: &citro3d_sys::C3D_Light) -> &Self {
        unsafe { (l as *const _ as *const Self).as_ref().unwrap() }
    }
    fn from_raw_mut(l: &mut citro3d_sys::C3D_Light) -> &mut Self {
        unsafe { (l as *mut _ as *mut Self).as_mut().unwrap() }
    }
    fn as_raw(&self) -> &citro3d_sys::C3D_Light {
        &self.raw
    }

    fn as_raw_mut(self: Pin<&mut Self>) -> &mut citro3d_sys::C3D_Light {
        unsafe { &mut self.get_unchecked_mut().raw }
    }
    pub fn set_position(self: Pin<&mut Self>, p: FVec3) {
        let mut p = FVec4::new(p.x(), p.y(), p.z(), 1.0);
        unsafe { citro3d_sys::C3D_LightPosition(self.as_raw_mut(), &mut p.0) }
    }
    pub fn set_color(self: Pin<&mut Self>, r: f32, g: f32, b: f32) {
        unsafe { citro3d_sys::C3D_LightColor(self.as_raw_mut(), r, g, b) }
    }
    #[doc(alias = "C3D_LightEnable")]
    pub fn set_enabled(self: Pin<&mut Self>, enabled: bool) {
        unsafe { citro3d_sys::C3D_LightEnable(self.as_raw_mut(), enabled) }
    }
    #[doc(alias = "C3D_LightShadowEnable")]
    pub fn set_shadow(self: Pin<&mut Self>, shadow: bool) {
        unsafe { citro3d_sys::C3D_LightShadowEnable(self.as_raw_mut(), shadow) }
    }
}

// Safety: I am 99% sure these are safe. That 1% is if citro3d does something weird I missed
// which is not impossible
unsafe impl Send for Light {}
unsafe impl Sync for Light {}

unsafe impl Send for LightEnv {}
unsafe impl Sync for LightEnv {}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct LutData(citro3d_sys::C3D_LightLut);

impl PartialEq for LutData {
    fn eq(&self, other: &Self) -> bool {
        self.0.data == other.0.data
    }
}
impl Eq for LutData {}

#[cfg(test)]
extern "C" fn c_powf(a: f32, b: f32) -> f32 {
    a.powf(b)
}

const LUT_SZ: usize = 512;

impl LutData {
    pub fn from_fn(mut f: impl FnMut(f32) -> f32, negative: bool) -> Self {
        let base: i32 = 128;
        let diff = if negative { 0 } else { base };
        let min = -128 + diff;
        let max = base + diff;
        assert_eq!(min.abs_diff(max), 2 * base as u32);
        let mut data = [0.0f32; LUT_SZ];
        for i in min..=max {
            let x = i as f32 / max as f32;
            let v = f(x);
            let idx = if negative { i & 0xFF } else { i } as usize;
            if i < max {
                data[idx] = v;
            }
            if i > min {
                data[idx + 255] = v - data[idx - 1];
            }
        }
        let lut = unsafe {
            let mut lut = MaybeUninit::zeroed();
            citro3d_sys::LightLut_FromArray(lut.as_mut_ptr(), data.as_mut_ptr());
            lut.assume_init()
        };
        Self(lut)
    }

    #[cfg(test)]
    fn phong_citro3d(shininess: f32) -> Self {
        let lut = unsafe {
            let mut lut = MaybeUninit::uninit();
            citro3d_sys::LightLut_FromFunc(lut.as_mut_ptr(), Some(c_powf), shininess, false);
            lut.assume_init()
        };
        Self(lut)
    }
}

/// This is used to decide what the input should be to a [`LutData`]
#[repr(u32)]
pub enum LutInput {
    CosPhi = ctru_sys::GPU_LUTINPUT_CP,
    /// Dot product of the light and normal vectors
    LightNormal = ctru_sys::GPU_LUTINPUT_LN,
    /// Half the normal
    NormalHalf = ctru_sys::GPU_LUTINPUT_NH,
    /// Dot product of the view and normal
    NormalView = ctru_sys::GPU_LUTINPUT_NV,
    /// Dot product of the spotlight colour and light vector
    LightSpotLight = ctru_sys::GPU_LUTINPUT_SP,
    /// Half the view vector
    ViewHalf = ctru_sys::GPU_LUTINPUT_VH,
}

#[repr(u32)]
pub enum LightLutId {
    D0 = ctru_sys::GPU_LUT_D0,
    D1 = ctru_sys::GPU_LUT_D1,
    SpotLightAttenuation = ctru_sys::GPU_LUT_SP,
    Fresnel = ctru_sys::GPU_LUT_FR,
    ReflectBlue = ctru_sys::GPU_LUT_RB,
    ReflectGreen = ctru_sys::GPU_LUT_RG,
    ReflectRed = ctru_sys::GPU_LUT_RR,
    DistanceAttenuation = ctru_sys::GPU_LUT_DA,
}

type LightArray = PinArray<Option<Light>, NB_LIGHTS>;

#[cfg(test)]
mod tests {
    use super::LutData;

    #[test]
    fn lut_data_phong_matches_for_own_and_citro3d() {
        let c3d = LutData::phong_citro3d(30.0);
        let rs = LutData::from_fn(|i| i.powf(30.0), false);
        assert_eq!(c3d, rs);
    }
}
