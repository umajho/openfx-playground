//! Builds the OpenFX plugin and bundles it according to the OpenFX specification, optionally taking care of producing
//! a universal binary for macOS.
//! For more information, see https://openfx.readthedocs.io/en/main/Reference/ofxPackaging.html.

use crate::build_plugin_common;
use crate::util::targets::Target;
use crate::util::{PathBufExt, workspace_dir};

use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

const COMMAND_NAME: &str = "learning-ofx-guide-1-build-plugin";
const PLUGIN_NAME: &str = "ofx-guide-1";
const PACKAGE_NAME: &str = "ofx-guide-1-basic-machinery";
const LIBRARY_NAME: &str = "ofx_guide_1_basic_machinery";
const BUNDLE_NAME: &str = "OfxGuide1";
const ICON_FILE_NAME: &str = "org.openeffects.BasicExamplePlugin";

static CRATE_DIR: OnceLock<PathBuf> = OnceLock::new();
fn crate_dir() -> &'static PathBuf {
    CRATE_DIR.get_or_init(|| {
        workspace_dir()
            .plus_iter(["crates", "learning", "ofx-guide-1-basic-machinery"])
            .to_path_buf()
    })
}

pub fn command() -> clap::Command {
    build_plugin_common::command(COMMAND_NAME, PLUGIN_NAME, crate_dir())
}

/// Creates the contents of the Info.plist file for the bundle when building for macOS.
fn get_info_plist() -> plist::Value {
    build_plugin_common::build_info_plist(
        &build_plugin_common::Info {
            bundle_identifier: "umajho.openfx-playground.learning.guide-1",
            human_readable_copyright: "© Umaĵo",
        },
        crate_dir(),
    )
}

/// Build the plugin for a given target, in either debug or release mode. This is called once in most cases, but when
/// creating a macOS universal binary, it's called twice--once per architecture.
/// This returns the path to the built library.
fn build_plugin_for_target(target: &Target, release_mode: bool) -> std::io::Result<PathBuf> {
    build_plugin_common::build_plugin_for_target(PACKAGE_NAME, LIBRARY_NAME, target, release_mode)
}

pub fn main(args: &clap::ArgMatches) -> Result<(), Box<dyn Error>> {
    let opts = build_plugin_common::BuildOptions::from_clap_arg_matches(args);

    build_plugin_common::build(
        &opts,
        build_plugin_for_target,
        get_info_plist(),
        crate_dir().plus_iter(["assets"]),
        BUNDLE_NAME,
        ICON_FILE_NAME,
    )
}
