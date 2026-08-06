//! ## TODO
//!
//! - [ ] initialzing tracing subscriber (The fact this this is a dynamic
//!   library should be taken into account.)

use std::{
    ffi::{CStr, c_char, c_int, c_void},
    sync::{Mutex, OnceLock},
};

use openfx_bindings::bindings::{
    OfxHost, OfxImageEffectHandle, OfxImageEffectSuiteV1, OfxPlugin, OfxPropertySetHandle,
    OfxPropertySetStruct, OfxPropertySuiteV1, OfxResult, OfxStat, OfxStatus,
    kOfxActionCreateInstance, kOfxActionDescribe, kOfxActionDestroyInstance, kOfxActionLoad,
    kOfxActionUnload, kOfxImageComponentAlpha, kOfxImageComponentRGBA,
    kOfxImageEffectActionDescribeInContext, kOfxImageEffectActionIsIdentity,
    kOfxImageEffectContextFilter, kOfxImageEffectPluginApi, kOfxImageEffectPluginPropGrouping,
    kOfxImageEffectPropContext, kOfxImageEffectPropSupportedComponents,
    kOfxImageEffectPropSupportedContexts, kOfxImageEffectSuite, kOfxPropInstanceData,
    kOfxPropLabel, kOfxPropName, kOfxPropertySuite,
};

#[unsafe(no_mangle)]
pub extern "C" fn OfxGetNumberOfPlugins() -> c_int {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn OfxGetPlugin(nth: c_int) -> *const OfxPlugin {
    if nth == 0 {
        return &EFFECT_PLUGIN_STRUCT;
    }

    std::ptr::null()
}

static EFFECT_PLUGIN_STRUCT: OfxPlugin = OfxPlugin {
    pluginApi: kOfxImageEffectPluginApi.as_ptr(),
    apiVersion: 1,
    pluginIdentifier: c"org.openeffects:BasicExamplePlugin".as_ptr(),
    pluginVersionMajor: 1,
    pluginVersionMinor: 0,
    setHost: Some(set_host),
    mainEntry: Some(main_entry),
};

static HOST_STRUCT: OnceLock<SaferHostStruct<'static>> = OnceLock::new();
#[derive(Clone)]
struct SaferHostStruct<'a> {
    host: &'a OfxPropertySetStruct,
    fetch_suite: unsafe extern "C" fn(
        host: OfxPropertySetHandle,
        suite_name: *const c_char,
        suite_version: c_int,
    ) -> *const c_void,
}

static SHARED_DATA: Mutex<Option<SharedData<'static>>> = Mutex::new(None);
struct SharedData<'a> {
    #[expect(unused)]
    host_struct: SaferHostStruct<'a>,
    property_suite: &'a OfxPropertySuiteV1,
    image_effect_suite: &'a OfxImageEffectSuiteV1,
}

unsafe extern "C" fn set_host(host_struct: *mut OfxHost) {
    fn inner(host_struct: *mut OfxHost) -> Result<(), &'static str> {
        let host_struct = unsafe {
            host_struct
                .as_mut()
                .ok_or("`host_struct` should not be null.")?
        };
        let host = unsafe {
            host_struct
                .host
                .as_mut()
                .ok_or("`host_struct.host` should not be null.")?
        };
        let fetch_suite = host_struct
            .fetchSuite
            .ok_or("`host_struct.fetchSuite` should not be null.")?;

        if HOST_STRUCT
            .set(SaferHostStruct { host, fetch_suite })
            .is_err()
        {
            return Err("`HOST_STRUCT` has already been initialized before.");
        }
        Ok(())
    }

    match inner(host_struct) {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("Failed to set host: {}", err);
        }
    }
}

unsafe extern "C" fn main_entry(
    action: *const c_char,
    handle: *const c_void,
    in_args: OfxPropertySetHandle,
    out_args: OfxPropertySetHandle,
) -> OfxStatus {
    let effect = handle as OfxImageEffectHandle;
    let action = if action.is_null() {
        return OfxStat::kOfxStatReplyDefault;
    } else {
        unsafe { CStr::from_ptr(action) }
    };

    let result = match true {
        _ if action == kOfxActionLoad => action_load(),
        _ if action == kOfxActionUnload => action_unload(),
        _ if action == kOfxActionDescribe => action_describe(effect),
        _ if action == kOfxImageEffectActionDescribeInContext => {
            action_describe_in_context(effect, in_args)
        }
        _ if action == kOfxActionCreateInstance => action_create_instance(effect),
        _ if action == kOfxActionDestroyInstance => action_destroy_instance(effect),
        _ if action == kOfxImageEffectActionIsIdentity => {
            action_is_identity(effect, in_args, out_args)
        }
        _ => OfxResult::Err(OfxStat::kOfxStatReplyDefault),
    };

    match result {
        Ok(_) => OfxStat::kOfxStatOK,
        Err(status) => status,
    }
}

fn action_load() -> OfxResult<()> {
    let host_struct = HOST_STRUCT.get().ok_or(OfxStat::kOfxStatErrFatal)?.clone();

    let property_suite = unsafe {
        (host_struct.fetch_suite)(
            host_struct.host as *const _ as OfxPropertySetHandle,
            kOfxPropertySuite.as_ptr(),
            1,
        )
    } as *const OfxPropertySuiteV1;
    let property_suite = unsafe {
        property_suite
            .as_ref()
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?
    };

    let image_effect_suite = unsafe {
        (host_struct.fetch_suite)(
            host_struct.host as *const _ as OfxPropertySetHandle,
            kOfxImageEffectSuite.as_ptr(),
            1,
        )
    } as *const OfxImageEffectSuiteV1;
    let image_effect_suite = unsafe {
        image_effect_suite
            .as_ref()
            .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?
    };

    let mut shared_data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    if shared_data.is_some() {
        Err(OfxStat::kOfxStatErrFatal)
    } else {
        *shared_data = Some(SharedData {
            host_struct,
            property_suite,
            image_effect_suite,
        });
        Ok(())
    }
}

fn action_unload() -> OfxResult<()> {
    let mut shared_data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    if shared_data.take().is_none() {
        Err(OfxStat::kOfxStatErrFatal)
    } else {
        Ok(())
    }
}

fn action_describe(descriptor: OfxImageEffectHandle) -> OfxResult<()> {
    let data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    let data = data.as_ref().ok_or(OfxStat::kOfxStatErrFatal)?;

    let get_property_set = data
        .image_effect_suite
        .getPropertySet
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

    let mut effect_props = std::ptr::null_mut();
    (unsafe { get_property_set(descriptor, &mut effect_props) }).ofx_ok()?;

    let prop_set_string = data
        .property_suite
        .propSetString
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

    unsafe {
        prop_set_string(
            effect_props,
            kOfxPropLabel.as_ptr(),
            0,
            c"OFX Basics Example".as_ptr(),
        )
        .ofx_ok()?;
        prop_set_string(
            effect_props,
            kOfxImageEffectPluginPropGrouping.as_ptr(),
            0,
            c"OFX Example".as_ptr(),
        )
        .ofx_ok()?;
        prop_set_string(
            effect_props,
            kOfxImageEffectPropSupportedContexts.as_ptr(),
            0,
            kOfxImageEffectContextFilter.as_ptr(),
        )
        .ofx_ok()?;
    }

    Ok(())
}

fn action_describe_in_context(
    descriptor: OfxImageEffectHandle,
    in_args: OfxPropertySetHandle,
) -> OfxResult<()> {
    let data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    let data = data.as_ref().ok_or(OfxStat::kOfxStatErrFatal)?;

    let prop_get_string = data
        .property_suite
        .propGetString
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;
    let prop_set_string = data
        .property_suite
        .propSetString
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;
    let clip_define = data
        .image_effect_suite
        .clipDefine
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

    let mut context: *mut c_char = std::ptr::null_mut();
    unsafe {
        prop_get_string(
            in_args,
            kOfxImageEffectPropContext.as_ptr(),
            0,
            &mut context,
        )
        .ofx_ok()?;
    }
    let context = unsafe { CStr::from_ptr(context) };
    if context != kOfxImageEffectContextFilter {
        return Err(OfxStat::kOfxStatErrUnsupported);
    }

    let mut props: *mut OfxPropertySetStruct = std::ptr::null_mut();
    unsafe {
        clip_define(descriptor, c"Output".as_ptr(), &mut props).ofx_ok()?;
        prop_set_string(
            props,
            kOfxImageEffectPropSupportedComponents.as_ptr(),
            0,
            kOfxImageComponentRGBA.as_ptr(),
        )
        .ofx_ok()?;
        prop_set_string(
            props,
            kOfxImageEffectPropSupportedComponents.as_ptr(),
            1,
            kOfxImageComponentAlpha.as_ptr(),
        )
        .ofx_ok()?;
    }

    let mut props: *mut OfxPropertySetStruct = std::ptr::null_mut();
    unsafe {
        clip_define(descriptor, c"Source".as_ptr(), &mut props).ofx_ok()?;
        prop_set_string(
            props,
            kOfxImageEffectPropSupportedComponents.as_ptr(),
            0,
            kOfxImageComponentRGBA.as_ptr(),
        )
        .ofx_ok()?;
        prop_set_string(
            props,
            kOfxImageEffectPropSupportedComponents.as_ptr(),
            1,
            kOfxImageComponentAlpha.as_ptr(),
        )
        .ofx_ok()?;
    }

    Ok(())
}

fn action_create_instance(instance: OfxImageEffectHandle) -> OfxResult<()> {
    let data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    let data = data.as_ref().ok_or(OfxStat::kOfxStatErrFatal)?;

    let get_property_set = data
        .image_effect_suite
        .getPropertySet
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;
    let prop_set_pointer = data
        .property_suite
        .propSetPointer
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

    let mut effect_props: *mut OfxPropertySetStruct = std::ptr::null_mut();
    (unsafe { get_property_set(instance, &mut effect_props) }).ofx_ok()?;

    let my_string = Box::new(String::from(
        "This is random instance data that could be anything you want.",
    ));
    let my_string = Box::into_raw(my_string) as *mut c_void;
    unsafe {
        // FIXME: `my_string` is leaked if this fails.
        if let Err(err) =
            prop_set_pointer(effect_props, kOfxPropInstanceData.as_ptr(), 0, my_string).ofx_ok()
        {
            drop(Box::from_raw(my_string.cast::<String>()));
            return Err(err);
        }
    }

    Ok(())
}

fn action_destroy_instance(instance: OfxImageEffectHandle) -> OfxResult<()> {
    let data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    let data = data.as_ref().ok_or(OfxStat::kOfxStatErrFatal)?;

    let get_property_set = data
        .image_effect_suite
        .getPropertySet
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;
    let prop_get_pointer = data
        .property_suite
        .propGetPointer
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;

    let mut effect_props: *mut OfxPropertySetStruct = std::ptr::null_mut();
    (unsafe { get_property_set(instance, &mut effect_props) }).ofx_ok()?;

    let mut my_string: *mut c_void = std::ptr::null_mut();
    (unsafe {
        prop_get_pointer(
            effect_props,
            kOfxPropInstanceData.as_ptr(),
            0,
            &mut my_string,
        )
    })
    .ofx_ok()?;

    // assert!(!my_string.is_null(), "Instance data should not be null!");

    drop(unsafe { Box::from_raw(my_string.cast::<String>()) });

    Ok(())
}

fn action_is_identity(
    _instance: OfxImageEffectHandle,
    _in_args: OfxPropertySetHandle,
    out_args: OfxPropertySetHandle,
) -> OfxResult<()> {
    let data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    let data = data.as_ref().ok_or(OfxStat::kOfxStatErrFatal)?;

    let prop_set_string = data
        .property_suite
        .propSetString
        .ok_or(OfxStat::kOfxStatErrMissingHostFeature)?;
    unsafe {
        prop_set_string(out_args, kOfxPropName.as_ptr(), 0, c"Source".as_ptr()).ofx_ok()?;
    }

    Ok(())
}
