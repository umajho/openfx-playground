use std::ffi::{CStr, c_char, c_int, c_void};

use openfx_bindings::bindings::{
    OfxImageClipHandle, OfxImageEffectHandle, OfxPropertySetHandle, OfxPropertySetStruct, OfxRectD,
    OfxResult, OfxTime,
};

use crate::SharedData;

pub struct SharedDataHelper<'a> {
    shared_data: &'a SharedData<'a>,
}

impl<'a> SharedDataHelper<'a> {
    pub fn try_new(shared_data: &'a SharedData<'a>) -> OfxResult<Self> {
        Ok(Self { shared_data })
    }

    pub fn inner(&self) -> &SharedData<'a> {
        self.shared_data
    }

    pub fn make_property_set_helper_for_image_effect<'helper>(
        &'helper self,
        handle: OfxImageEffectHandle,
    ) -> OfxResult<PropertySetHelper<'helper, 'a>> {
        let get_property_set = self
            .shared_data
            .image_effect_suite
            .getPropertySet
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut props: *mut OfxPropertySetStruct = std::ptr::null_mut();
        (unsafe { get_property_set(handle, &mut props) }).ofx_ok()?;

        Ok(self.make_property_set_helper(props as OfxPropertySetHandle))
    }

    pub fn make_property_set_helper<'helper>(
        &'helper self,
        handle: OfxPropertySetHandle,
    ) -> PropertySetHelper<'helper, 'a> {
        PropertySetHelper {
            shared_data_helper: self,
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
            .shared_data
            .property_suite
            .propSetString
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

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
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

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
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

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
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

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
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

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
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut value: f64 = 0.0;
        (unsafe { prop_get_double(handle, property.as_ptr(), index, &mut value) }).ofx_ok()?;

        Ok(value)
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
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

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
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

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
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

        let mut clip: OfxImageClipHandle = std::ptr::null_mut();
        (unsafe { clip_get_handle(image_effect, name.as_ptr(), &mut clip, std::ptr::null_mut()) })
            .ofx_ok()?;

        Ok(clip)
    }

    fn clip_get_image(
        &self,
        clip: OfxImageClipHandle,
        time: OfxTime,
        region: Option<*const OfxRectD>,
    ) -> OfxResult<OfxPropertySetHandle> {
        let clip_get_image = self
            .shared_data
            .image_effect_suite
            .clipGetImage
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

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
    ) -> OfxResult<ClipImageManaged<'helper, 'a>> {
        let image_handle = self.clip_get_image(clip, time, region)?;

        Ok(ClipImageManaged {
            shared_data_helper: self,
            image_handle,
        })
    }

    pub fn clip_release_image(&self, image_handle: OfxPropertySetHandle) -> OfxResult<()> {
        let clip_release_image = self
            .shared_data
            .image_effect_suite
            .clipReleaseImage
            .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)?;

        (unsafe { clip_release_image(image_handle) }).ofx_ok()
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

    pub fn prop_get_double(&self, property: &CStr, index: c_int) -> OfxResult<f64> {
        self.shared_data_helper
            .prop_get_double(self.props, property, index)
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
            .clip_release_image(self.image_handle);
    }
}
