//! Some functions in this module that are not marked as `unsafe` may actually
//! be unsafe. (FIXME)

#![allow(clippy::not_unsafe_ptr_arg_deref)] // TODO: reconsider this.

use std::ffi::{c_char, c_int, c_void, CStr};

use crate::{
    bindings::{
        kOfxBitDepthByte, kOfxBitDepthFloat, kOfxBitDepthShort, kOfxImageComponentAlpha,
        kOfxImageComponentRGB, kOfxImageComponentRGBA, kOfxImageEffectPropComponents,
        kOfxImageEffectPropPixelDepth, kOfxImagePropBounds, kOfxImagePropData,
        kOfxImagePropRowBytes, OfxImageClipHandle, OfxImageEffectHandle, OfxImageEffectSuiteV1,
        OfxParamHandle, OfxParamSetHandle, OfxPropertySetHandle, OfxPropertySetStruct,
        OfxPropertySuiteV1, OfxRectD, OfxRectI, OfxResult, OfxStat, OfxTime,
    },
    helpers::internal_utils::rect_i_from_array,
};

use super::SharedData;

pub struct SharedDataHelper<'data> {
    shared_data: &'data SharedData<'data>,
}

impl<'data> SharedDataHelper<'data> {
    pub fn try_new(shared_data: &'data SharedData<'data>) -> OfxResult<Self> {
        Ok(Self { shared_data })
    }

    pub fn inner(&self) -> &SharedData<'data> {
        self.shared_data
    }

    pub fn property_suite_helper(&self) -> PropertySuiteHelper<'data> {
        PropertySuiteHelper {
            property_suite: self.shared_data.property_suite,
        }
    }
    pub fn image_effect_suite_helper(&self) -> ImageEffectSuiteHelper<'data> {
        ImageEffectSuiteHelper {
            image_effect_suite: self.shared_data.image_effect_suite,
        }
    }
    pub fn parameter_suite_helper(&self) -> ParameterSuiteHelper<'data> {
        ParameterSuiteHelper {
            parameter_suite: self.shared_data.parameter_suite,
        }
    }

    pub fn make_property_set_helper_for_image_effect(
        &self,
        handle: OfxImageEffectHandle,
    ) -> OfxResult<PropertySetHelper<'data>> {
        let props = self.image_effect_suite_helper().get_property_set(handle)?;

        Ok(self.property_suite_helper().make_property_set_helper(props))
    }

    pub fn make_param_set_helper_for_image_effect(
        &self,
        handle: OfxImageEffectHandle,
    ) -> OfxResult<ParamSetHelper<'data>> {
        let param_set = self.image_effect_suite_helper().get_param_set(handle)?;

        Ok(self
            .parameter_suite_helper()
            .make_param_set_helper(param_set))
    }

    pub fn make_clip_image_managed(
        &self,
        clip: OfxImageClipHandle,
        time: OfxTime,
        region: Option<*const OfxRectD>,
    ) -> OfxResult<Option<ClipImageManaged<'data>>> {
        let image_handle = self
            .image_effect_suite_helper()
            .clip_get_image(clip, time, region)?;

        ClipImageManaged::try_new(self, image_handle)
    }
}

pub struct PropertySuiteHelper<'data> {
    property_suite: &'data OfxPropertySuiteV1,
}

impl<'data> PropertySuiteHelper<'data> {
    pub fn inner(&self) -> &OfxPropertySuiteV1 {
        self.property_suite
    }

    pub fn make_property_set_helper(
        &self,
        handle: OfxPropertySetHandle,
    ) -> PropertySetHelper<'data> {
        PropertySetHelper {
            property_suite_helper: PropertySuiteHelper {
                property_suite: self.property_suite,
            },
            props: handle,
        }
    }

    pub fn prop_set_string(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
        value: &CStr,
    ) -> OfxResult<()> {
        let prop_set_string = self
            .property_suite
            .propSetString
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        (unsafe { prop_set_string(handle, property.as_ptr(), index, value.as_ptr()) }).ofx_ok()
    }

    pub fn prop_get_string(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
    ) -> OfxResult<Option<&CStr>> {
        let prop_get_string = self
            .property_suite
            .propGetString
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut value_ptr: *mut c_char = std::ptr::null_mut();
        (unsafe { prop_get_string(handle, property.as_ptr(), index, &mut value_ptr) }).ofx_ok()?;

        if value_ptr.is_null() {
            return Ok(None);
        }

        let value_cstr = unsafe { CStr::from_ptr(value_ptr) };
        Ok(Some(value_cstr))
    }

    pub fn prop_set_int(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
        value: c_int,
    ) -> OfxResult<()> {
        let prop_set_int = self
            .property_suite
            .propSetInt
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        (unsafe { prop_set_int(handle, property.as_ptr(), index, value) }).ofx_ok()
    }

    pub fn prop_get_int(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
    ) -> OfxResult<c_int> {
        let prop_get_int = self
            .property_suite
            .propGetInt
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut value: c_int = 0;
        (unsafe { prop_get_int(handle, property.as_ptr(), index, &mut value) }).ofx_ok()?;

        Ok(value)
    }

    pub fn prop_get_int_n(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        values: &mut [c_int],
    ) -> OfxResult<()> {
        let prop_get_int_n = self
            .property_suite
            .propGetIntN
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        (unsafe {
            prop_get_int_n(
                handle,
                property.as_ptr(),
                values.len() as c_int,
                values.as_mut_ptr(),
            )
        })
        .ofx_ok()
    }

    pub fn prop_set_double(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
        value: f64,
    ) -> OfxResult<()> {
        let prop_set_double = self
            .property_suite
            .propSetDouble
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        (unsafe { prop_set_double(handle, property.as_ptr(), index, value) }).ofx_ok()
    }

    pub fn prop_get_double(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
    ) -> OfxResult<f64> {
        let prop_get_double = self
            .property_suite
            .propGetDouble
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut value: f64 = 0.0;
        (unsafe { prop_get_double(handle, property.as_ptr(), index, &mut value) }).ofx_ok()?;

        Ok(value)
    }

    pub fn prop_set_pointer(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
        value: *mut c_void,
    ) -> OfxResult<()> {
        let prop_set_pointer = self
            .property_suite
            .propSetPointer
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        (unsafe { prop_set_pointer(handle, property.as_ptr(), index, value) }).ofx_ok()
    }

    pub fn prop_get_pointer(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
    ) -> OfxResult<*mut c_void> {
        let prop_get_pointer = self
            .property_suite
            .propGetPointer
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut value_ptr: *mut c_void = std::ptr::null_mut();
        (unsafe { prop_get_pointer(handle, property.as_ptr(), index, &mut value_ptr) }).ofx_ok()?;

        Ok(value_ptr)
    }
}

pub struct PropertySetHelper<'data> {
    property_suite_helper: PropertySuiteHelper<'data>,
    props: OfxPropertySetHandle,
}

impl<'data> PropertySetHelper<'data> {
    pub fn props(&self) -> OfxPropertySetHandle {
        self.props
    }

    pub fn prop_set_string(&self, property: &CStr, index: c_int, value: &CStr) -> OfxResult<()> {
        self.property_suite_helper
            .prop_set_string(self.props, property, index, value)
    }

    pub fn prop_get_string(&self, property: &CStr, index: c_int) -> OfxResult<Option<&CStr>> {
        self.property_suite_helper
            .prop_get_string(self.props, property, index)
    }

    pub fn prop_set_int(&self, property: &CStr, index: c_int, value: c_int) -> OfxResult<()> {
        self.property_suite_helper
            .prop_set_int(self.props, property, index, value)
    }

    pub fn prop_get_int(&self, property: &CStr, index: c_int) -> OfxResult<c_int> {
        self.property_suite_helper
            .prop_get_int(self.props, property, index)
    }

    pub fn prop_get_int_n(&self, property: &CStr, values: &mut [c_int]) -> OfxResult<()> {
        self.property_suite_helper
            .prop_get_int_n(self.props, property, values)
    }

    pub fn prop_set_double(&self, property: &CStr, index: c_int, value: f64) -> OfxResult<()> {
        self.property_suite_helper
            .prop_set_double(self.props, property, index, value)
    }

    pub fn prop_get_double(&self, property: &CStr, index: c_int) -> OfxResult<f64> {
        self.property_suite_helper
            .prop_get_double(self.props, property, index)
    }

    pub fn prop_set_pointer(
        &self,
        property: &CStr,
        index: c_int,
        value: *mut c_void,
    ) -> OfxResult<()> {
        self.property_suite_helper
            .prop_set_pointer(self.props, property, index, value)
    }

    pub fn prop_get_pointer(&self, property: &CStr, index: c_int) -> OfxResult<*mut c_void> {
        self.property_suite_helper
            .prop_get_pointer(self.props, property, index)
    }
}

pub struct ImageEffectSuiteHelper<'data> {
    image_effect_suite: &'data OfxImageEffectSuiteV1,
}

impl<'data> ImageEffectSuiteHelper<'data> {
    pub fn inner(&self) -> &OfxImageEffectSuiteV1 {
        self.image_effect_suite
    }

    fn get_property_set(&self, handle: OfxImageEffectHandle) -> OfxResult<OfxPropertySetHandle> {
        let get_property_set = self
            .image_effect_suite
            .getPropertySet
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut props: *mut OfxPropertySetStruct = std::ptr::null_mut();
        (unsafe { get_property_set(handle, &mut props) }).ofx_ok()?;

        Ok(props as OfxPropertySetHandle)
    }

    pub fn clip_define(
        &self,
        image_effect: OfxImageEffectHandle,
        name: &CStr,
    ) -> OfxResult<OfxPropertySetHandle> {
        let clip_define = self
            .image_effect_suite
            .clipDefine
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut props: OfxPropertySetHandle = std::ptr::null_mut();
        (unsafe { clip_define(image_effect, name.as_ptr(), &mut props) }).ofx_ok()?;

        Ok(props)
    }

    pub fn clip_get_handle(
        &self,
        image_effect: OfxImageEffectHandle,
        name: &CStr,
    ) -> OfxResult<OfxImageClipHandle> {
        let clip_get_handle = self
            .image_effect_suite
            .clipGetHandle
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut clip: OfxImageClipHandle = std::ptr::null_mut();
        (unsafe { clip_get_handle(image_effect, name.as_ptr(), &mut clip, std::ptr::null_mut()) })
            .ofx_ok()?;

        Ok(clip)
    }

    /// Use [`Self::clip_get_image_managed`].
    pub fn clip_get_image(
        &self,
        clip: OfxImageClipHandle,
        time: OfxTime,
        region: Option<*const OfxRectD>,
    ) -> OfxResult<OfxPropertySetHandle> {
        let clip_get_image = self
            .image_effect_suite
            .clipGetImage
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut image: OfxPropertySetHandle = std::ptr::null_mut();
        (unsafe { clip_get_image(clip, time, region.unwrap_or(std::ptr::null()), &mut image) })
            .ofx_ok()?;

        Ok(image)
    }

    /// Use [`Self::clip_get_image_managed`] to get a managed image that does
    /// not require calling this function manually to release it.
    pub fn clip_release_image_(&self, image_handle: OfxPropertySetHandle) -> OfxResult<()> {
        let clip_release_image = self
            .image_effect_suite
            .clipReleaseImage
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        (unsafe { clip_release_image(image_handle) }).ofx_ok()
    }

    pub fn get_param_set(
        &self,
        image_effect: OfxImageEffectHandle,
    ) -> OfxResult<OfxParamSetHandle> {
        let get_param_set = self
            .image_effect_suite
            .getParamSet
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut param_set: OfxParamSetHandle = std::ptr::null_mut();
        (unsafe { get_param_set(image_effect, &mut param_set) }).ofx_ok()?;

        Ok(param_set)
    }
}

pub struct ClipImageManaged<'data> {
    image_effect_suite_helper: ImageEffectSuiteHelper<'data>,
    image_handle: OfxPropertySetHandle,

    n_comps: c_int,
    pixel_depth: BitDepth,
    row_bytes: c_int,
    bounds: OfxRectI,
    data_ptr: *mut c_void,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BitDepth {
    Byte,
    Short,
    Float,
}

impl<'data> ClipImageManaged<'data> {
    fn try_new(
        shared_data_helper: &SharedDataHelper<'data>,
        image_handle: OfxPropertySetHandle,
    ) -> OfxResult<Option<Self>> {
        let s_prop = shared_data_helper.property_suite_helper();

        let props = s_prop.make_property_set_helper(image_handle);

        let data_ptr = props.prop_get_pointer(kOfxImagePropData, 0)?;
        if data_ptr.is_null() {
            return Ok(None);
        }

        let n_comps = {
            let components = props.prop_get_string(kOfxImageEffectPropComponents, 0)?;
            match components {
                Some(c) if c == kOfxImageComponentRGBA => 4,
                Some(c) if c == kOfxImageComponentRGB => 3,
                Some(c) if c == kOfxImageComponentAlpha => 1,
                _ => 0,
            }
        };
        let pixel_depth = {
            let pixel_depth = props.prop_get_string(kOfxImageEffectPropPixelDepth, 0)?;
            match true {
                _ if pixel_depth == Some(kOfxBitDepthByte) => BitDepth::Byte,
                _ if pixel_depth == Some(kOfxBitDepthShort) => BitDepth::Short,
                _ if pixel_depth == Some(kOfxBitDepthFloat) => BitDepth::Float,
                _ => return Err(OfxStat::kOfxStatErrUnsupported),
            }
        };
        let row_bytes = props.prop_get_int(kOfxImagePropRowBytes, 0)?;
        let bounds = {
            let mut bounds: [c_int; 4] = [0; 4];
            props.prop_get_int_n(kOfxImagePropBounds, &mut bounds)?;
            rect_i_from_array(&bounds)
        };

        Ok(Some(Self {
            image_effect_suite_helper: shared_data_helper.image_effect_suite_helper(),
            image_handle,

            n_comps,
            pixel_depth,
            row_bytes,
            bounds,
            data_ptr,
        }))
    }

    /// ## Safety
    ///
    /// The caller must ensure that the returned `OfxPropertySetHandle` will not
    /// be used after the `ClipImageManaged` instance is dropped.
    pub fn image_handle(&self) -> OfxPropertySetHandle {
        self.image_handle
    }

    pub fn n_comps(&self) -> c_int {
        self.n_comps
    }
    pub fn pixel_depth(&self) -> BitDepth {
        self.pixel_depth
    }
    pub fn bytes_per_component(&self) -> c_int {
        match self.pixel_depth {
            BitDepth::Byte => 1,
            BitDepth::Short => 2,
            BitDepth::Float => 4,
        }
    }
    pub fn bytes_per_pixel(&self) -> c_int {
        self.bytes_per_component() * self.n_comps
    }
    pub fn row_bytes(&self) -> c_int {
        self.row_bytes
    }
    pub fn bounds(&self) -> OfxRectI {
        self.bounds
    }
    pub fn data_ptr(&self) -> *mut c_void {
        self.data_ptr
    }

    pub fn raw_address(&self, x: c_int, y: c_int) -> Option<*mut c_void> {
        if x < self.bounds.x1 || x >= self.bounds.x2 || y < self.bounds.y1 || y >= self.bounds.y2 {
            return None;
        }

        let x_offset = x - self.bounds.x1;
        let y_offset = y - self.bounds.y1;

        let row_start = unsafe {
            (self.data_ptr as *mut u8).offset(y_offset as isize * self.row_bytes as isize)
        };

        Some(unsafe {
            row_start.offset(x_offset as isize * self.bytes_per_pixel() as isize) as *mut c_void
        })
    }
}

impl<'data> Drop for ClipImageManaged<'data> {
    fn drop(&mut self) {
        let _ = self
            .image_effect_suite_helper
            .clip_release_image_(self.image_handle);
    }
}

pub struct ParameterSuiteHelper<'data> {
    parameter_suite: &'data crate::bindings::OfxParameterSuiteV1,
}

impl<'data> ParameterSuiteHelper<'data> {
    pub fn inner(&self) -> &crate::bindings::OfxParameterSuiteV1 {
        self.parameter_suite
    }

    pub fn make_param_set_helper(&self, handle: OfxParamSetHandle) -> ParamSetHelper<'data> {
        ParamSetHelper {
            parameter_suite_helper: ParameterSuiteHelper {
                parameter_suite: self.parameter_suite,
            },
            param_set: handle,
        }
    }

    pub fn param_define(
        &self,
        param_set: OfxParamSetHandle,
        param_type: &CStr,
        name: &CStr,
    ) -> OfxResult<OfxPropertySetHandle> {
        let param_define = self
            .parameter_suite
            .paramDefine
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut props: OfxPropertySetHandle = std::ptr::null_mut();
        (unsafe { param_define(param_set, param_type.as_ptr(), name.as_ptr(), &mut props) })
            .ofx_ok()?;

        Ok(props)
    }

    pub fn param_get_handle(
        &self,
        param_set: OfxParamSetHandle,
        name: &CStr,
    ) -> OfxResult<OfxParamHandle> {
        let param_get_handle = self
            .parameter_suite
            .paramGetHandle
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut props: OfxParamHandle = std::ptr::null_mut();
        (unsafe { param_get_handle(param_set, name.as_ptr(), &mut props, std::ptr::null_mut()) })
            .ofx_ok()?;

        Ok(props)
    }

    /// ## Safety
    ///
    /// TODO
    fn param_get_value_at_time<T>(
        &self,
        param_handle: OfxParamHandle,
        time: OfxTime,
    ) -> OfxResult<T> {
        let param_get_value_at_time = self
            .parameter_suite
            .paramGetValueAtTime
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut value: T = unsafe { std::mem::zeroed() };
        (unsafe { param_get_value_at_time(param_handle, time, &mut value) }).ofx_ok()?;

        Ok(value)
    }

    pub fn param_get_value_at_time_double(
        &self,
        param_handle: OfxParamHandle,
        time: OfxTime,
    ) -> OfxResult<f64> {
        self.param_get_value_at_time(param_handle, time)
    }

    pub fn param_get_value_at_time_int(
        &self,
        param_handle: OfxParamHandle,
        time: OfxTime,
    ) -> OfxResult<c_int> {
        self.param_get_value_at_time(param_handle, time)
    }
}

pub struct ParamSetHelper<'data> {
    parameter_suite_helper: ParameterSuiteHelper<'data>,
    param_set: OfxParamSetHandle,
}

impl<'data> ParamSetHelper<'data> {
    pub fn param_set(&self) -> OfxParamSetHandle {
        self.param_set
    }

    pub fn param_define(&self, param_type: &CStr, name: &CStr) -> OfxResult<OfxPropertySetHandle> {
        self.parameter_suite_helper
            .param_define(self.param_set, param_type, name)
    }

    pub fn param_get_handle(&self, name: &CStr) -> OfxResult<OfxParamHandle> {
        self.parameter_suite_helper
            .param_get_handle(self.param_set, name)
    }
}
