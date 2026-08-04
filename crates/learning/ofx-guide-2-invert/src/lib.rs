#![feature(once_cell_try)]

//! ## TODO
//!
//! - [ ] initialzing tracing subscriber (The fact this this is a dynamic
//!       library should be taken into account.)

mod shared_data_helper;

use std::{
    ffi::{CStr, c_char, c_int, c_void},
    sync::{Mutex, OnceLock},
};

use openfx_bindings::bindings::{
    OfxHost, OfxImageEffectHandle, OfxImageEffectSuiteV1, OfxPlugin, OfxPropertySetHandle,
    OfxPropertySetStruct, OfxPropertySuiteV1, OfxRectI, OfxResult, OfxStat, OfxStatus, OfxTime,
    kOfxActionCreateInstance, kOfxActionDescribe, kOfxActionDestroyInstance, kOfxActionLoad,
    kOfxActionUnload, kOfxBitDepthByte, kOfxBitDepthFloat, kOfxBitDepthShort,
    kOfxImageComponentAlpha, kOfxImageComponentRGB, kOfxImageComponentRGBA,
    kOfxImageEffectActionDescribeInContext, kOfxImageEffectActionRender,
    kOfxImageEffectContextFilter, kOfxImageEffectPluginApi, kOfxImageEffectPluginPropGrouping,
    kOfxImageEffectPluginPropHostFrameThreading, kOfxImageEffectPluginRenderThreadSafety,
    kOfxImageEffectPropComponents, kOfxImageEffectPropContext, kOfxImageEffectPropPixelDepth,
    kOfxImageEffectPropRenderWindow, kOfxImageEffectPropSupportedComponents,
    kOfxImageEffectPropSupportedContexts, kOfxImageEffectPropSupportedPixelDepths,
    kOfxImageEffectRenderFullySafe, kOfxImageEffectSuite, kOfxImagePropBounds, kOfxImagePropData,
    kOfxImagePropRowBytes, kOfxPropLabel, kOfxPropTime, kOfxPropertySuite,
};

use crate::shared_data_helper::SharedDataHelper;

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
    pluginIdentifier: c"org.openeffects:InvertExamplePlugin".as_ptr(),
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
#[derive(Clone)]
struct SharedData<'a> {
    #[expect(unused)]
    host_struct: SaferHostStruct<'a>,
    property_suite: &'a OfxPropertySuiteV1,
    image_effect_suite: &'a OfxImageEffectSuiteV1,
}

fn shared_data_lockless() -> OfxResult<SharedData<'static>> {
    let data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    let data = data.as_ref().ok_or(OfxStat::kOfxStatErrFatal)?;
    Ok(data.clone())
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
        _ if action == kOfxImageEffectActionRender => action_render(effect, in_args, out_args),
        _ if action == kOfxActionCreateInstance || action == kOfxActionDestroyInstance => {
            // We need to handle these actions (even if it's just a no-op) for DaVinci resolve to properly load our plugin
            // If not handled, it'll load the plugin but will never show the controls or actually render anything
            Ok(())
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

    let mut data = SHARED_DATA.lock().map_err(|_| OfxStat::kOfxStatErrFatal)?;
    if data.is_some() {
        Err(OfxStat::kOfxStatErrFatal)
    } else {
        *data = Some(SharedData {
            host_struct,
            property_suite,
            image_effect_suite,
        });
        Ok(())
    }
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
    let data = shared_data_lockless()?;
    let mut data = SharedDataHelper::try_new(&data)?;

    let mut descriptor_helper = data.make_property_set_helper_for_image_effect(descriptor)?;

    descriptor_helper.prop_set_string(kOfxPropLabel, 0, c"OFX Invert Example")?;
    descriptor_helper.prop_set_string(kOfxImageEffectPluginPropGrouping, 0, c"OFX Example")?;
    descriptor_helper.prop_set_string(
        kOfxImageEffectPropSupportedContexts,
        0,
        kOfxImageEffectContextFilter,
    )?;

    for (i, bp) in [kOfxBitDepthFloat, kOfxBitDepthShort, kOfxBitDepthByte]
        .iter()
        .enumerate()
    {
        descriptor_helper.prop_set_string(
            kOfxImageEffectPropSupportedPixelDepths,
            i as c_int,
            bp,
        )?;
    }

    descriptor_helper.prop_set_string(
        kOfxImageEffectPluginRenderThreadSafety,
        0,
        kOfxImageEffectRenderFullySafe,
    )?;
    descriptor_helper.prop_set_int(kOfxImageEffectPluginPropHostFrameThreading, 0, 1)?;

    Ok(())
}

fn action_describe_in_context(
    descriptor: OfxImageEffectHandle,
    in_args: OfxPropertySetHandle,
) -> OfxResult<()> {
    let data = shared_data_lockless()?;
    let mut data = SharedDataHelper::try_new(&data)?;

    let mut in_args_helper = data.make_property_set_helper(in_args);

    let context = in_args_helper.prop_get_string(kOfxImageEffectPropContext, 0)?;
    if context != Some(kOfxImageEffectContextFilter) {
        return Err(OfxStat::kOfxStatErrUnsupported);
    }

    for name in [c"Output", c"Source"] {
        let props = data.clip_define(descriptor, name)?;
        let mut props_helper = data.make_property_set_helper(props);

        for (i, comp) in [
            kOfxImageComponentRGBA,
            kOfxImageComponentAlpha,
            kOfxImageComponentRGB,
        ]
        .iter()
        .enumerate()
        {
            props_helper.prop_set_string(
                kOfxImageEffectPropSupportedComponents,
                i as c_int,
                comp,
            )?;
        }
    }

    Ok(())
}

/// Look up a pixel in the image. Returns `None` if the pixel was not in the
/// bounds of the image.
fn pixel_address<T>(
    x: c_int,
    y: c_int,
    base_address: *mut T,
    bounds: OfxRectI,
    row_bytes: c_int,
    n_comps_per_pixel: c_int,
) -> Option<*mut T> {
    if x < bounds.x1 || x >= bounds.x2 || y < bounds.y1 || y >= bounds.y2 {
        return None;
    }

    let x_offset = x - bounds.x1;
    let y_offset = y - bounds.y1;

    let row_start_address =
        unsafe { (base_address as *mut u8).offset((y_offset * row_bytes) as isize) as *mut T };

    Some(unsafe { row_start_address.offset((x_offset * n_comps_per_pixel) as isize) })
}

fn pixel_processing<T>(
    max: T,
    data: &mut SharedDataHelper,
    instance: OfxImageEffectHandle,
    source_img: OfxPropertySetHandle,
    output_img: OfxPropertySetHandle,
    render_window: OfxRectI,
    n_comps: c_int,
) -> OfxResult<()>
where
    T: std::ops::Sub<Output = T> + Copy + Default,
{
    let mut output_img_helper = data.make_property_set_helper(output_img);
    let dst_row_bytes = output_img_helper.prop_get_int(kOfxImagePropRowBytes, 0)?;
    let dst_bounds = {
        let mut dst_bounds: [c_int; 4] = [0; 4];
        output_img_helper.prop_get_int_n(kOfxImagePropBounds, &mut dst_bounds)?;
        rect_i_from_array(&dst_bounds)
    };
    let dst_ptr = output_img_helper.prop_get_pointer(kOfxImagePropData, 0)? as *mut T;
    if dst_ptr.is_null() {
        return Err(OfxStat::kOfxStatErrBadIndex);
    }

    let mut source_img_helper = data.make_property_set_helper(source_img);
    let src_row_bytes = source_img_helper.prop_get_int(kOfxImagePropRowBytes, 0)?;
    let src_bounds = {
        let mut src_bounds: [c_int; 4] = [0; 4];
        source_img_helper.prop_get_int_n(kOfxImagePropBounds, &mut src_bounds)?;
        rect_i_from_array(&src_bounds)
    };
    let src_ptr = source_img_helper.prop_get_pointer(kOfxImagePropData, 0)? as *mut T;
    if src_ptr.is_null() {
        return Err(OfxStat::kOfxStatErrBadIndex);
    }

    for y in render_window.y1..render_window.y2 {
        if y % 20 == 0
            && data
                .inner()
                .image_effect_suite
                .abort
                .is_some_and(|abort| unsafe { abort(instance) } != 0)
        {
            return Err(OfxStat::kOfxStatOK);
        }

        let Some(dst_pix) = pixel_address(
            render_window.x1,
            y,
            dst_ptr,
            dst_bounds,
            dst_row_bytes,
            n_comps,
        ) else {
            continue;
        };
        let mut dst_pix = dst_pix;

        for x in render_window.x1..render_window.x2 {
            let src_pix = pixel_address(x, y, src_ptr, src_bounds, src_row_bytes, n_comps);

            if let Some(src_pix) = src_pix {
                let mut src_pix = src_pix;
                for i in 0..n_comps {
                    unsafe {
                        *dst_pix = if i != 3 { max - *src_pix } else { *src_pix };
                        dst_pix = dst_pix.offset(1);
                        src_pix = src_pix.offset(1);
                    }
                }
            } else {
                for _ in 0..n_comps {
                    unsafe {
                        *dst_pix = T::default();
                        dst_pix = dst_pix.offset(1);
                    }
                }
            }
        }
    }

    Ok(())
}

fn rect_i_from_array(arr: &[c_int; 4]) -> OfxRectI {
    OfxRectI {
        x1: arr[0],
        y1: arr[1],
        x2: arr[2],
        y2: arr[3],
    }
}

fn action_render(
    instance: OfxImageEffectHandle,
    in_args: OfxPropertySetHandle,
    _out_args: OfxPropertySetHandle,
) -> OfxResult<()> {
    let data = shared_data_lockless()?;
    let mut data = SharedDataHelper::try_new(&data)?;

    let mut in_args_helper = data.make_property_set_helper(in_args);

    let time: OfxTime = in_args_helper.prop_get_double(kOfxPropTime, 0)?;
    let render_window = {
        let mut render_window: [c_int; 4] = [0; 4];
        in_args_helper.prop_get_int_n(kOfxImageEffectPropRenderWindow, &mut render_window)?;
        rect_i_from_array(&render_window)
    };

    let output_clip = data.clip_get_handle(instance, c"Output")?;
    let source_clip = data.clip_get_handle(instance, c"Source")?;

    let output_img = data.clip_get_image(output_clip, time, None)?;
    let source_img = data.clip_get_image(source_clip, time, None)?;

    fn inner(
        data: &mut SharedDataHelper,
        instance: OfxImageEffectHandle,
        source_img: OfxPropertySetHandle,
        output_img: OfxPropertySetHandle,
        render_window: OfxRectI,
    ) -> OfxResult<()> {
        let mut output_img_helper = data.make_property_set_helper(output_img);

        let components = output_img_helper.prop_get_string(kOfxImageEffectPropComponents, 0)?;
        let n_comps = match components {
            Some(c) if c == kOfxImageComponentRGBA => 4,
            Some(c) if c == kOfxImageComponentRGB => 3,
            Some(c) if c == kOfxImageComponentAlpha => 1,
            _ => return Err(OfxStat::kOfxStatErrUnsupported),
        };

        let data_type = output_img_helper.prop_get_string(kOfxImageEffectPropPixelDepth, 0)?;
        match data_type {
            Some(c) if c == kOfxBitDepthByte => pixel_processing(
                255u8,
                data,
                instance,
                source_img,
                output_img,
                render_window,
                n_comps,
            ),
            Some(c) if c == kOfxBitDepthShort => pixel_processing(
                65535u16,
                data,
                instance,
                source_img,
                output_img,
                render_window,
                n_comps,
            ),
            Some(c) if c == kOfxBitDepthFloat => pixel_processing(
                1.0f32,
                data,
                instance,
                source_img,
                output_img,
                render_window,
                n_comps,
            ),
            _ => return Err(OfxStat::kOfxStatErrUnsupported),
        }?;

        Ok(())
    }

    let result = inner(&mut data, instance, source_img, output_img, render_window);

    data.clip_release_image(output_img)?;
    data.clip_release_image(source_img)?;

    result
}
