use std::ffi::{CStr, c_char, c_int, c_void};

use openfx_bindings::bindings::{
    OfxImageClipHandle, OfxImageEffectHandle, OfxParamHandle, OfxParamSetHandle,
    OfxPropertySetHandle, OfxPropertySetStruct, OfxRectD, OfxResult, OfxStat, OfxTime,
};

use crate::SharedData;

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

    pub fn make_property_set_helper_for_image_effect<'helper>(
        &'helper self,
        handle: OfxImageEffectHandle,
    ) -> OfxResult<PropertySetHelper<'helper, 'data>> {
        let props = self.get_property_set(handle)?;

        Ok(self.make_property_set_helper(props as OfxPropertySetHandle))
    }

    pub fn make_property_set_helper<'helper>(
        &'helper self,
        handle: OfxPropertySetHandle,
    ) -> PropertySetHelper<'helper, 'data> {
        PropertySetHelper {
            shared_data_helper: self,
            props: handle,
        }
    }

    fn get_property_set(&self, handle: OfxImageEffectHandle) -> OfxResult<OfxPropertySetHandle> {
        let get_property_set = self
            .shared_data
            .image_effect_suite
            .getPropertySet
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut props: *mut OfxPropertySetStruct = std::ptr::null_mut();
        (unsafe { get_property_set(handle, &mut props) }).ofx_ok()?;

        Ok(props as OfxPropertySetHandle)
    }

    pub fn prop_set_string(
        &self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
        value: &CStr,
    ) -> OfxResult<()> {
        let prop_set_string = self
            .shared_data
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
            .shared_data
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
            .shared_data
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
            .shared_data
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
            .shared_data
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
            .shared_data
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
            .shared_data
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
            .shared_data
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
            .shared_data
            .property_suite
            .propGetPointer
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut value_ptr: *mut c_void = std::ptr::null_mut();
        (unsafe { prop_get_pointer(handle, property.as_ptr(), index, &mut value_ptr) }).ofx_ok()?;

        Ok(value_ptr)
    }

    pub fn clip_define(
        &self,
        image_effect: OfxImageEffectHandle,
        name: &CStr,
    ) -> OfxResult<OfxPropertySetHandle> {
        let clip_define = self
            .shared_data
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
            .shared_data
            .image_effect_suite
            .clipGetHandle
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut clip: OfxImageClipHandle = std::ptr::null_mut();
        (unsafe { clip_get_handle(image_effect, name.as_ptr(), &mut clip, std::ptr::null_mut()) })
            .ofx_ok()?;

        Ok(clip)
    }

    /// Use [`Self::clip_get_image_managed`].
    pub fn clip_get_image_(
        &self,
        clip: OfxImageClipHandle,
        time: OfxTime,
        region: Option<*const OfxRectD>,
    ) -> OfxResult<OfxPropertySetHandle> {
        let clip_get_image = self
            .shared_data
            .image_effect_suite
            .clipGetImage
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut image: OfxPropertySetHandle = std::ptr::null_mut();
        (unsafe { clip_get_image(clip, time, region.unwrap_or(std::ptr::null()), &mut image) })
            .ofx_ok()?;

        Ok(image)
    }

    pub fn clip_get_image_managed<'helper>(
        &'helper self,
        clip: OfxImageClipHandle,
        time: OfxTime,
        region: Option<*const OfxRectD>,
    ) -> OfxResult<ClipImageManaged<'helper, 'data>> {
        let image_handle = self.clip_get_image_(clip, time, region)?;

        Ok(ClipImageManaged {
            shared_data_helper: self,
            image_handle,
        })
    }

    /// Use [`Self::clip_get_image_managed`] to get a managed image that does
    /// not require calling this function manually to release it.
    pub fn clip_release_image_(&self, image_handle: OfxPropertySetHandle) -> OfxResult<()> {
        let clip_release_image = self
            .shared_data
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
            .shared_data
            .image_effect_suite
            .getParamSet
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut param_set: OfxParamSetHandle = std::ptr::null_mut();
        (unsafe { get_param_set(image_effect, &mut param_set) }).ofx_ok()?;

        Ok(param_set)
    }

    pub fn param_define(
        &self,
        param_set: OfxParamSetHandle,
        param_type: &CStr,
        name: &CStr,
    ) -> OfxResult<OfxPropertySetHandle> {
        let param_define = self
            .shared_data
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
            .shared_data
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
            .shared_data
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

pub struct PropertySetHelper<'helper, 'data> {
    shared_data_helper: &'helper SharedDataHelper<'data>,
    props: OfxPropertySetHandle,
}

impl<'helper, 'data> PropertySetHelper<'helper, 'data> {
    pub fn prop_set_string(&self, property: &CStr, index: c_int, value: &CStr) -> OfxResult<()> {
        self.shared_data_helper
            .prop_set_string(self.props, property, index, value)
    }

    pub fn prop_get_string(&self, property: &CStr, index: c_int) -> OfxResult<Option<&CStr>> {
        self.shared_data_helper
            .prop_get_string(self.props, property, index)
    }

    pub fn prop_set_int(&self, property: &CStr, index: c_int, value: c_int) -> OfxResult<()> {
        self.shared_data_helper
            .prop_set_int(self.props, property, index, value)
    }

    pub fn prop_get_int(&self, property: &CStr, index: c_int) -> OfxResult<c_int> {
        self.shared_data_helper
            .prop_get_int(self.props, property, index)
    }

    pub fn prop_get_int_n(&self, property: &CStr, values: &mut [c_int]) -> OfxResult<()> {
        self.shared_data_helper
            .prop_get_int_n(self.props, property, values)
    }

    pub fn prop_set_double(&self, property: &CStr, index: c_int, value: f64) -> OfxResult<()> {
        self.shared_data_helper
            .prop_set_double(self.props, property, index, value)
    }

    pub fn prop_get_double(&self, property: &CStr, index: c_int) -> OfxResult<f64> {
        self.shared_data_helper
            .prop_get_double(self.props, property, index)
    }

    pub fn prop_set_pointer(
        &self,
        property: &CStr,
        index: c_int,
        value: *mut c_void,
    ) -> OfxResult<()> {
        self.shared_data_helper
            .prop_set_pointer(self.props, property, index, value)
    }

    pub fn prop_get_pointer(&self, property: &CStr, index: c_int) -> OfxResult<*mut c_void> {
        self.shared_data_helper
            .prop_get_pointer(self.props, property, index)
    }
}

pub struct ClipImageManaged<'helper, 'data> {
    shared_data_helper: &'helper SharedDataHelper<'data>,
    image_handle: OfxPropertySetHandle,
}

impl<'helper, 'data> ClipImageManaged<'helper, 'data> {
    /// ## Safety
    ///
    /// The caller must ensure that the returned `OfxPropertySetHandle` will not
    /// be used after the `ClipImageManaged` instance is dropped.
    pub fn image_handle(&self) -> OfxPropertySetHandle {
        self.image_handle
    }
}

impl Drop for ClipImageManaged<'_, '_> {
    fn drop(&mut self) {
        let _ = self
            .shared_data_helper
            .clip_release_image_(self.image_handle);
    }
}
