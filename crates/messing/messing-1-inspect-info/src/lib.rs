//! ## TODO
//!
//! - [ ] initializing tracing subscriber (The fact that this is a dynamic
//!   library should be taken into account.)
//!
//! ## FIXME
//!
//! This only works in Natron. In Davinci Resolve, it reports: “No parameter is
//! exposed to user or OFX Plugin [org.openeffects:circleexampleplugin] is not
//! available.”

use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    sync::{Arc, Mutex, OnceLock},
};

use openfx_bindings::{
    bindings::{
        OfxHost, OfxImageClipHandle, OfxImageEffectHandle, OfxParamHandle, OfxPlugin,
        OfxPropertySetHandle, OfxResult, OfxStat, OfxStatus, kOfxActionCreateInstance,
        kOfxActionDescribe, kOfxActionDestroyInstance, kOfxActionLoad, kOfxActionUnload,
        kOfxBitDepthByte, kOfxBitDepthFloat, kOfxBitDepthShort, kOfxImageComponentRGB,
        kOfxImageComponentRGBA, kOfxImageEffectActionDescribeInContext,
        kOfxImageEffectActionIsIdentity, kOfxImageEffectContextFilter, kOfxImageEffectPluginApi,
        kOfxImageEffectPluginPropGrouping, kOfxImageEffectPluginPropHostFrameThreading,
        kOfxImageEffectPluginRenderThreadSafety, kOfxImageEffectPropContext,
        kOfxImageEffectPropSupportedComponents, kOfxImageEffectPropSupportedContexts,
        kOfxImageEffectPropSupportedPixelDepths, kOfxImageEffectRenderFullySafe,
        kOfxParamPropDefault, kOfxParamPropEnabled, kOfxParamPropHint, kOfxParamTypeString,
        kOfxPropAPIVersion, kOfxPropInstanceData, kOfxPropLabel, kOfxPropName,
    },
    helpers::{SaferHostStruct, SharedData, shared_data_helper::SharedDataHelper},
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
    pluginIdentifier: c"umajho.openeffects:messing.inspect-info".as_ptr(),
    pluginVersionMajor: 1,
    pluginVersionMinor: 0,
    setHost: Some(set_host),
    mainEntry: Some(main_entry),
};

static HOST_STRUCT: OnceLock<SaferHostStruct<'static>> = OnceLock::new();

static SHARED_DATA: Mutex<Option<(SharedData<'static>, Arc<AdditionalSharedData>)>> =
    Mutex::new(None);

struct AdditionalSharedData {
    api_version: [c_int; 2],
}

#[expect(unused)]
struct InstanceData {
    source_clip: OfxImageClipHandle,
    output_clip: OfxImageClipHandle,

    version_param: OfxParamHandle,
}

fn shared_data_lockless() -> OfxResult<(SharedData<'static>, Arc<AdditionalSharedData>)> {
    let data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    let data = data.as_ref().ok_or(OfxStat::kOfxStatErrFatal)?;
    Ok((data.0.clone(), data.1.clone()))
}

const VERSION_PARAM_NAME: &CStr = c"version";

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

    let mut data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    if data.is_some() {
        return Err(OfxStat::kOfxStatErrFatal);
    }

    *data = Some({
        let data = SharedData::try_new(host_struct)?;

        let additional = {
            let data = SharedDataHelper::try_new(&data)?;

            let host_props = data.make_property_set_helper_for_host()?;
            let var_size = host_props.prop_get_dimension(kOfxPropAPIVersion)?;
            let mut api_version = [1, 0];
            if var_size == 1 {
                api_version[0] = host_props.prop_get_int(kOfxPropAPIVersion, 0)?;
            } else {
                host_props.prop_get_int_n(kOfxPropAPIVersion, &mut api_version)?;
            }

            AdditionalSharedData { api_version }
        };

        (data, Arc::new(additional))
    });

    Ok(())
}

fn action_unload() -> OfxResult<()> {
    let mut data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    if data.take().is_none() {
        Err(OfxStat::kOfxStatErrFatal)
    } else {
        Ok(())
    }
}

fn action_describe(descriptor: OfxImageEffectHandle) -> OfxResult<()> {
    let (data, _additional) = shared_data_lockless()?;
    let data = SharedDataHelper::try_new(&data)?;

    let descriptor = data.make_property_set_helper_for_image_effect(descriptor)?;

    descriptor.prop_set_string(kOfxPropLabel, 0, c"Inspect Info")?;
    descriptor.prop_set_string(kOfxImageEffectPluginPropGrouping, 0, c"Umaĵo: Messing")?;

    descriptor.prop_set_string(
        kOfxImageEffectPropSupportedContexts,
        0,
        kOfxImageEffectContextFilter,
    )?;

    for (i, bp) in [kOfxBitDepthFloat, kOfxBitDepthShort, kOfxBitDepthByte]
        .iter()
        .enumerate()
    {
        descriptor.prop_set_string(kOfxImageEffectPropSupportedPixelDepths, i as c_int, bp)?;
    }

    descriptor.prop_set_string(
        kOfxImageEffectPluginRenderThreadSafety,
        0,
        kOfxImageEffectRenderFullySafe,
    )?;
    descriptor.prop_set_int(kOfxImageEffectPluginPropHostFrameThreading, 0, 1)?;

    Ok(())
}

fn action_describe_in_context(
    descriptor: OfxImageEffectHandle,
    in_args: OfxPropertySetHandle,
) -> OfxResult<()> {
    let (data, additional) = shared_data_lockless()?;
    let data = SharedDataHelper::try_new(&data)?;

    let s_prop = data.property_suite_helper();
    let s_ifx = data.image_effect_suite_helper();

    let in_args = s_prop.make_property_set_helper(in_args);

    let context = in_args.prop_get_string(kOfxImageEffectPropContext, 0)?;
    if context != Some(kOfxImageEffectContextFilter) {
        return Err(OfxStat::kOfxStatErrUnsupported);
    }

    for name in [c"Output", c"Source"] {
        let props = s_ifx.clip_define(descriptor, name)?;
        let props = s_prop.make_property_set_helper(props);

        props.prop_set_string_n(
            kOfxImageEffectPropSupportedComponents,
            &[
                kOfxImageComponentRGBA.as_ptr(),
                kOfxImageComponentRGB.as_ptr(),
            ],
        )?;
    }

    let param_set = data.make_param_set_helper_for_image_effect(descriptor)?;

    {
        let param_props = param_set.param_define(kOfxParamTypeString, VERSION_PARAM_NAME)?;
        let param_props = s_prop.make_property_set_helper(param_props);
        let api_version_string = CString::new(format!(
            "{}.{}",
            additional.api_version[0], additional.api_version[1]
        ))
        .map_err(|_| OfxStat::kOfxStatErrFatal)?;
        param_props.prop_set_string(kOfxParamPropDefault, 0, &api_version_string)?;
        param_props.prop_set_string(kOfxPropLabel, 0, c"OpenFX Version")?;
        param_props.prop_set_string(
            kOfxParamPropHint,
            0,
            c"The Version of OpenFX the host supports.",
        )?;
        param_props.prop_set_int(kOfxParamPropEnabled, 0, 0)?;
    }

    Ok(())
}

fn action_create_instance(instance: OfxImageEffectHandle) -> Result<(), OfxStatus> {
    let (data, _additional) = shared_data_lockless()?;
    let data = SharedDataHelper::try_new(&data)?;

    let s_ifx = data.image_effect_suite_helper();

    let instance_props = data.make_property_set_helper_for_image_effect(instance)?;

    let source_clip = s_ifx.clip_get_handle(instance, c"Source")?;
    let output_clip = s_ifx.clip_get_handle(instance, c"Output")?;

    let param_set = data.make_param_set_helper_for_image_effect(instance)?;
    let version_param = param_set.param_get_handle(VERSION_PARAM_NAME)?;

    let instance_data = InstanceData {
        source_clip,
        output_clip,
        version_param,
    };
    let instance_data_ptr = Box::into_raw(Box::new(instance_data)) as *mut c_void;

    match instance_props.prop_set_pointer(kOfxPropInstanceData, 0, instance_data_ptr) {
        Ok(_) => Ok(()),
        Err(err) => {
            drop(unsafe { Box::from_raw(instance_data_ptr as *mut InstanceData) });
            Err(err)
        }
    }
}

fn action_destroy_instance(instance: OfxImageEffectHandle) -> Result<(), OfxStatus> {
    let (data, _additional) = shared_data_lockless()?;
    let data = SharedDataHelper::try_new(&data)?;

    let instance_props = data.make_property_set_helper_for_image_effect(instance)?;
    let instance_data_ptr = instance_props.prop_get_pointer(kOfxPropInstanceData, 0)?;
    if instance_data_ptr.is_null() {
        return Err(OfxStat::kOfxStatErrFatal);
    }

    drop(unsafe { Box::from_raw(instance_data_ptr as *mut InstanceData) });

    Ok(())
}

fn action_is_identity(
    _effect: OfxImageEffectHandle,
    _in_args: OfxPropertySetHandle,
    out_args: OfxPropertySetHandle,
) -> Result<(), OfxStatus> {
    let (data, _additional) = shared_data_lockless()?;
    let data = SharedDataHelper::try_new(&data)?;

    let s_prop = data.property_suite_helper();

    let out_args = s_prop.make_property_set_helper(out_args);

    out_args.prop_set_string(kOfxPropName, 0, c"Source")?;
    Ok(())
}
