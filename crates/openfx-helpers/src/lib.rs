pub mod shared_data_helper;

use std::ffi::{c_char, c_int, c_void};

use openfx_bindings::bindings::{
    OfxImageEffectSuiteV1, OfxPropertySetHandle, OfxPropertySetStruct, OfxPropertySuiteV1,
};

#[derive(Clone)]
pub struct SaferHostStruct<'a> {
    pub host: &'a OfxPropertySetStruct,
    pub fetch_suite: unsafe extern "C" fn(
        host: OfxPropertySetHandle,
        suite_name: *const c_char,
        suite_version: c_int,
    ) -> *const c_void,
}

#[derive(Clone)]
pub struct SharedData<'a> {
    pub host_struct: SaferHostStruct<'a>,
    pub property_suite: &'a OfxPropertySuiteV1,
    pub image_effect_suite: &'a OfxImageEffectSuiteV1,
}
