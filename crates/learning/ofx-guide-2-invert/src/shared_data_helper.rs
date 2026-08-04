use std::{
    ffi::{CStr, c_char, c_int, c_void},
    sync::OnceLock,
};

use openfx_bindings::bindings::{
    OfxImageClipHandle, OfxImageEffectHandle, OfxPropertySetHandle, OfxPropertySetStruct, OfxRectD,
    OfxResult, OfxStatus, OfxTime,
};

use crate::SharedData;

pub struct SharedDataHelper<'a> {
    shared_data: &'a SharedData<'a>,

    prop_set_string: OnceLock<
        unsafe extern "C" fn(
            properties: OfxPropertySetHandle,
            property: *const c_char,
            index: c_int,
            value: *const c_char,
        ) -> OfxStatus,
    >,
    prop_get_string: OnceLock<
        unsafe extern "C" fn(
            properties: OfxPropertySetHandle,
            property: *const c_char,
            index: c_int,
            value: *mut *mut c_char,
        ) -> OfxStatus,
    >,
    prop_set_int: OnceLock<
        unsafe extern "C" fn(
            properties: OfxPropertySetHandle,
            property: *const c_char,
            index: c_int,
            value: c_int,
        ) -> OfxStatus,
    >,
    prop_get_int: OnceLock<
        unsafe extern "C" fn(
            properties: OfxPropertySetHandle,
            property: *const c_char,
            index: c_int,
            value: *mut c_int,
        ) -> OfxStatus,
    >,
    prop_get_int_n: OnceLock<
        unsafe extern "C" fn(
            properties: OfxPropertySetHandle,
            property: *const c_char,
            count: c_int,
            values: *mut c_int,
        ) -> OfxStatus,
    >,
    prop_get_pointer: OnceLock<
        unsafe extern "C" fn(
            properties: OfxPropertySetHandle,
            property: *const c_char,
            index: c_int,
            value: *mut *mut c_void,
        ) -> OfxStatus,
    >,
    clip_define: OnceLock<
        unsafe extern "C" fn(
            image_effect: OfxImageEffectHandle,
            name: *const c_char,
            properties: *mut OfxPropertySetHandle,
        ) -> OfxStatus,
    >,
    clip_get_handle: OnceLock<
        unsafe extern "C" fn(
            image_effect: OfxImageEffectHandle,
            name: *const c_char,
            clip: *mut OfxImageClipHandle,
            property_set: *mut OfxPropertySetHandle,
        ) -> OfxStatus,
    >,
    clip_get_image: OnceLock<
        unsafe extern "C" fn(
            clip: OfxImageClipHandle,
            time: OfxTime,
            region: *const OfxRectD,
            image: *mut OfxPropertySetHandle,
        ) -> OfxStatus,
    >,
    clip_release_image:
        OnceLock<unsafe extern "C" fn(image_handle: OfxPropertySetHandle) -> OfxStatus>,
}

impl<'a> SharedDataHelper<'a> {
    pub fn try_new(shared_data: &'a SharedData<'a>) -> OfxResult<Self> {
        Ok(Self {
            shared_data,
            prop_set_string: OnceLock::new(),
            prop_get_string: OnceLock::new(),
            prop_set_int: OnceLock::new(),
            prop_get_int: OnceLock::new(),
            prop_get_int_n: OnceLock::new(),
            prop_get_pointer: OnceLock::new(),
            clip_define: OnceLock::new(),
            clip_get_handle: OnceLock::new(),
            clip_get_image: OnceLock::new(),
            clip_release_image: OnceLock::new(),
        })
    }

    pub fn inner(&self) -> &SharedData<'a> {
        self.shared_data
    }

    pub fn make_property_set_helper_for_image_effect<'helper>(
        &'helper mut self,
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
        &'helper mut self,
        handle: OfxPropertySetHandle,
    ) -> PropertySetHelper<'helper, 'a> {
        PropertySetHelper {
            shared_data_helper: self,
            props: handle,
        }
    }

    pub fn prop_set_string(
        &mut self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
        value: &CStr,
    ) -> OfxResult<()> {
        let prop_set_string = self.prop_set_string.get_or_try_init(|| {
            self.shared_data
                .property_suite
                .propSetString
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        (unsafe { prop_set_string(handle, property.as_ptr(), index, value.as_ptr()) }).ofx_ok()
    }

    pub fn prop_get_string(
        &mut self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
    ) -> OfxResult<Option<&CStr>> {
        let prop_get_string = self.prop_get_string.get_or_try_init(|| {
            self.shared_data
                .property_suite
                .propGetString
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        let mut value_ptr: *mut c_char = std::ptr::null_mut();
        (unsafe { prop_get_string(handle, property.as_ptr(), index, &mut value_ptr) }).ofx_ok()?;

        if value_ptr.is_null() {
            return Ok(None);
        }

        let value_cstr = unsafe { CStr::from_ptr(value_ptr) };
        Ok(Some(value_cstr))
    }

    pub fn prop_set_int(
        &mut self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
        value: c_int,
    ) -> OfxResult<()> {
        let prop_set_int = self.prop_set_int.get_or_try_init(|| {
            self.shared_data
                .property_suite
                .propSetInt
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        (unsafe { prop_set_int(handle, property.as_ptr(), index, value) }).ofx_ok()
    }

    pub fn prop_get_int(
        &mut self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
    ) -> OfxResult<c_int> {
        let prop_get_int = self.prop_get_int.get_or_try_init(|| {
            self.shared_data
                .property_suite
                .propGetInt
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        let mut value: c_int = 0;
        (unsafe { prop_get_int(handle, property.as_ptr(), index, &mut value) }).ofx_ok()?;

        Ok(value)
    }

    pub fn prop_get_int_n(
        &mut self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        values: &mut [c_int],
    ) -> OfxResult<()> {
        let prop_get_int_n = self.prop_get_int_n.get_or_try_init(|| {
            self.shared_data
                .property_suite
                .propGetIntN
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

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
        &mut self,
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
        &mut self,
        handle: OfxPropertySetHandle,
        property: &CStr,
        index: c_int,
    ) -> OfxResult<*mut c_void> {
        let prop_get_pointer = self.prop_get_pointer.get_or_try_init(|| {
            self.shared_data
                .property_suite
                .propGetPointer
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        let mut value_ptr: *mut c_void = std::ptr::null_mut();
        (unsafe { prop_get_pointer(handle, property.as_ptr(), index, &mut value_ptr) }).ofx_ok()?;

        Ok(value_ptr)
    }

    pub fn clip_define(
        &mut self,
        image_effect: OfxImageEffectHandle,
        name: &CStr,
    ) -> OfxResult<OfxPropertySetHandle> {
        let clip_define = self.clip_define.get_or_try_init(|| {
            self.shared_data
                .image_effect_suite
                .clipDefine
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        let mut props: OfxPropertySetHandle = std::ptr::null_mut();
        (unsafe { clip_define(image_effect, name.as_ptr(), &mut props) }).ofx_ok()?;

        Ok(props)
    }

    pub fn clip_get_handle(
        &mut self,
        image_effect: OfxImageEffectHandle,
        name: &CStr,
    ) -> OfxResult<OfxImageClipHandle> {
        let clip_get_handle = self.clip_get_handle.get_or_try_init(|| {
            self.shared_data
                .image_effect_suite
                .clipGetHandle
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        let mut clip: OfxImageClipHandle = std::ptr::null_mut();
        (unsafe { clip_get_handle(image_effect, name.as_ptr(), &mut clip, std::ptr::null_mut()) })
            .ofx_ok()?;

        Ok(clip)
    }

    pub fn clip_get_image(
        &mut self,
        clip: OfxImageClipHandle,
        time: OfxTime,
        region: Option<*const OfxRectD>,
    ) -> OfxResult<OfxPropertySetHandle> {
        let clip_get_image = self.clip_get_image.get_or_try_init(|| {
            self.shared_data
                .image_effect_suite
                .clipGetImage
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        let mut image: OfxPropertySetHandle = std::ptr::null_mut();
        (unsafe { clip_get_image(clip, time, region.unwrap_or(std::ptr::null()), &mut image) })
            .ofx_ok()?;

        Ok(image)
    }

    pub fn clip_release_image(&mut self, image_handle: OfxPropertySetHandle) -> OfxResult<()> {
        let clip_release_image = self.clip_release_image.get_or_try_init(|| {
            self.shared_data
                .image_effect_suite
                .clipReleaseImage
                .ok_or(openfx_bindings::bindings::OfxStat::kOfxStatErrMissingHostFeature)
        })?;

        (unsafe { clip_release_image(image_handle) }).ofx_ok()
    }
}

pub struct PropertySetHelper<'helper, 'data> {
    shared_data_helper: &'helper mut SharedDataHelper<'data>,
    props: OfxPropertySetHandle,
}

impl<'helper, 'data> PropertySetHelper<'helper, 'data> {
    pub fn prop_set_string(
        &mut self,
        property: &CStr,
        index: c_int,
        value: &CStr,
    ) -> OfxResult<()> {
        self.shared_data_helper
            .prop_set_string(self.props, property, index, value)
    }

    pub fn prop_get_string(&mut self, property: &CStr, index: c_int) -> OfxResult<Option<&CStr>> {
        self.shared_data_helper
            .prop_get_string(self.props, property, index)
    }

    pub fn prop_set_int(&mut self, property: &CStr, index: c_int, value: c_int) -> OfxResult<()> {
        self.shared_data_helper
            .prop_set_int(self.props, property, index, value)
    }

    pub fn prop_get_int(&mut self, property: &CStr, index: c_int) -> OfxResult<c_int> {
        self.shared_data_helper
            .prop_get_int(self.props, property, index)
    }

    pub fn prop_get_int_n(&mut self, property: &CStr, values: &mut [c_int]) -> OfxResult<()> {
        self.shared_data_helper
            .prop_get_int_n(self.props, property, values)
    }

    pub fn prop_get_double(&mut self, property: &CStr, index: c_int) -> OfxResult<f64> {
        self.shared_data_helper
            .prop_get_double(self.props, property, index)
    }

    pub fn prop_get_pointer(&mut self, property: &CStr, index: c_int) -> OfxResult<*mut c_void> {
        self.shared_data_helper
            .prop_get_pointer(self.props, property, index)
    }
}
