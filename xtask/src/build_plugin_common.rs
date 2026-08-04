use std::{
    error::Error,
    ffi::OsString,
    fs,
    hash::{BuildHasher, Hasher},
    path::PathBuf,
    process::Command,
};

use clap::builder::PathBufValueParser;

use crate::util::{
    PathBufExt as _, StatusExt as _,
    targets::{MACOS_AARCH64, MACOS_X86_64, Target},
    workspace_dir,
};

pub fn command(
    command_name: &str,
    plugin_name: &str,
    crate_dir: impl Into<PathBuf>,
) -> clap::Command {
    let crate_dir = crate_dir.into();
    let build_dir = crate_dir.plus_iter(["build"]);

    let about = format!(
        "Builds and bundles the {} OpenFX plugin, which is then output to `{}`.",
        plugin_name,
        build_dir.display()
    );

    clap::Command::new(command_name.to_owned())
        .about(&about)
        .arg(
            clap::Arg::new("release")
                .long("release")
                .help("Build the plugin in release mode")
                .conflicts_with("debug")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("debug")
                .long("debug")
                .help("Build the plugin in debug mode")
                .conflicts_with("release")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("target")
                .long("target")
                .help("Set the target triple to compile for")
                .default_value(current_platform::CURRENT_PLATFORM),
        )
        .arg(
            clap::Arg::new("macos-universal")
                .long("macos-universal")
                .help("Build a macOS universal library (x86_64 and aarch64)")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("target"),
        )
        .arg(
            clap::Arg::new("destdir")
                .long("destdir")
                .help("The directory that the OpenFX plugin bundle will be output to")
                .value_parser(PathBufValueParser::new())
                .default_value(build_dir.as_os_str().to_owned()),
        )
}

pub struct Info<'a> {
    pub bundle_identifier: &'a str,
    pub human_readable_copyright: &'a str,
}

/// Creates the contents of the Info.plist file for the bundle when building for macOS.
pub fn build_info_plist(info: &Info, crate_dir: impl Into<PathBuf>) -> plist::Value {
    let crate_dir = crate_dir.into();
    let cargo_toml_path = crate_dir.plus_iter(["Cargo.toml"]);
    let manifest = cargo_toml::Manifest::from_path(cargo_toml_path).unwrap();
    let version = manifest.package().version();

    let mut info_plist_contents = plist::dictionary::Dictionary::new();
    info_plist_contents.insert(
        "CFBundleInfoDictionaryVersion".to_string(),
        plist::Value::from("6.0"),
    );
    info_plist_contents.insert(
        "CFBundleDevelopmentRegion".to_string(),
        plist::Value::from("en"),
    );
    info_plist_contents.insert(
        "CFBundlePackageType".to_string(),
        plist::Value::from("BNDL"),
    );
    info_plist_contents.insert(
        "CFBundleIdentifier".to_string(),
        plist::Value::from(info.bundle_identifier.to_owned()),
    );
    info_plist_contents.insert(
        "CFBundleVersion".to_string(),
        plist::Value::from(version.to_string()),
    );
    info_plist_contents.insert(
        "CFBundleShortVersionString".to_string(),
        plist::Value::from(version.to_string()),
    );
    info_plist_contents.insert(
        "NSHumanReadableCopyright".to_string(),
        plist::Value::from(info.human_readable_copyright.to_owned()),
    );
    info_plist_contents.insert("CFBundleSignature".to_string(), plist::Value::from("????"));

    plist::Value::Dictionary(info_plist_contents)
}

/// Build the plugin for a given target, in either debug or release mode.
/// This returns the path to the built library.
pub fn build_plugin_for_target(
    package_name: &str,
    library_name: &str,
    target: &Target,
    release_mode: bool,
) -> std::io::Result<PathBuf> {
    println!("Building OpenFX plugin for target {}", target.target_triple);

    let mut cargo_args: Vec<_> = vec![
        String::from("build"),
        format!("--package={}", package_name),
        String::from("--lib"),
        String::from("--target"),
        target.target_triple.to_string(),
    ];
    if release_mode {
        cargo_args.push(String::from("--release"));
    }
    Command::new("cargo")
        .args(&cargo_args)
        .status()
        .expect_success()?;

    let target_dir_path = workspace_dir().to_path_buf().plus_iter([
        "target",
        target.target_triple,
        if cargo_args.contains(&String::from("--release")) {
            "release"
        } else {
            "debug"
        },
    ]);

    let mut built_library_path =
        target_dir_path.plus(target.library_prefix.to_owned() + library_name);
    built_library_path.set_extension(target.library_extension);

    Ok(built_library_path)
}

pub struct BuildOptions {
    pub release_mode: bool,
    pub macos_universal: bool,
    pub targets: Targets,
    pub output_dir: PathBuf,
}
pub enum Targets {
    MacOSUniversal,
    Single(Target),
}

impl BuildOptions {
    pub fn from_clap_arg_matches(args: &clap::ArgMatches) -> Self {
        let release_mode = args.get_flag("release");
        let macos_universal = args.get_flag("macos-universal");
        let output_dir = args.get_one::<PathBuf>("destdir").unwrap();

        let targets = if macos_universal {
            Targets::MacOSUniversal
        } else {
            let target_triple = args.get_one::<String>("target").unwrap();
            let target = Target::from_triple(target_triple).unwrap();
            Targets::Single(*target)
        };

        BuildOptions {
            release_mode,
            macos_universal,
            targets,
            output_dir: output_dir.to_owned(),
        }
    }
}

pub fn build(
    opts: &BuildOptions,
    build_plugin_for_target: impl Fn(&Target, bool) -> std::io::Result<PathBuf>,
    info_plist: plist::Value,
    assets_dir: impl Into<PathBuf>,
    bundle_name: &str,
    icon_file_name: &str,
) -> Result<(), Box<dyn Error>> {
    let assets_dir = assets_dir.into();

    // TODO: remove previous built bundle?

    let (built_library_path, ofx_architecture) = match opts.targets {
        Targets::MacOSUniversal => {
            let x86_64_target = MACOS_X86_64;
            let aarch64_target = MACOS_AARCH64;
            let x86_64_path = build_plugin_for_target(x86_64_target, opts.release_mode)?;
            let aarch64_path = build_plugin_for_target(aarch64_target, opts.release_mode)?;

            let dst_path = std::env::temp_dir().plus(format!(
                "openfx-playground-{}-{}",
                random_u64(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ));

            // Combine the x86_64 and aarch64 builds into one using `lipo`, and output to the temp file we created
            // above.
            // TODO: Create the directories beforehand, output into that with lipo, and just rename it afterwards?
            Command::new("lipo")
                .args(&[
                    OsString::from("-create"),
                    OsString::from("-output"),
                    dst_path.clone().into(),
                    x86_64_path.into(),
                    aarch64_path.into(),
                ])
                .status()
                .expect_success()?;

            // Both targets should have ofx_architecture: "MacOS" since it's a universal binary. Some platforms have
            // different bundle directories depending on the architecture, but as of Apple Silicon, that's not done for
            // macOS:
            // https://openfx.readthedocs.io/en/main/Reference/ofxPackaging.html#macos-architectures-and-universal-binaries
            assert_eq!(
                x86_64_target.ofx_architecture,
                aarch64_target.ofx_architecture
            );
            (dst_path, x86_64_target.ofx_architecture)
        }
        Targets::Single(target) => (
            build_plugin_for_target(&target, opts.release_mode)?,
            target.ofx_architecture,
        ),
    };

    let output_dir = &opts.output_dir;

    let plugin_bundle_path =
        output_dir.plus_iter([&format!("{}.ofx.bundle", bundle_name), "Contents"]);
    let plugin_bin_path =
        plugin_bundle_path.plus_iter([ofx_architecture, &format!("{}.ofx", bundle_name)]);
    let plugin_resources_path = plugin_bundle_path.plus_iter(["Resources"]);

    fs::create_dir_all(plugin_bin_path.parent().unwrap())?;
    fs::create_dir_all(&plugin_resources_path)?;
    fs::copy(built_library_path, plugin_bin_path)?;
    if ofx_architecture == "MacOS" {
        info_plist.to_file_xml(plugin_bundle_path.plus("Info.plist"))?;
        fs::copy(
            assets_dir.plus_iter(["macos_icon.png"]),
            plugin_resources_path.plus(&format!("{}.png", icon_file_name)),
        )?;
    } else {
        fs::copy(
            assets_dir.plus_iter(["icon.png"]),
            plugin_resources_path.plus(&format!("{}.png", icon_file_name)),
        )?;
    }

    Ok(())
}

fn random_u64() -> u64 {
    std::hash::RandomState::new().build_hasher().finish()
}
