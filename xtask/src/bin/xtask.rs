//! cargo-xtask is a pattern which provides a platform-independent way to run build scripts by writing them in Rust.
//! While many of the build scripts are to some degree platform-specific, there's a lot of shared logic that is nice
//! to be able to reuse between platforms.
//! See https://github.com/matklad/cargo-xtask for more information.

use std::process;

use xtask::learning_ntsc_openfx_build_plugin;
use xtask::learning_ofx_guide_1_build_plugin;
use xtask::learning_ofx_guide_2_build_plugin;

fn main() {
    let cmd = clap::Command::new("xtask")
        .subcommand_required(true)
        .subcommand(learning_ntsc_openfx_build_plugin::command())
        .subcommand(learning_ofx_guide_1_build_plugin::command())
        .subcommand(learning_ofx_guide_2_build_plugin::command());

    let matches = cmd.get_matches();

    let (task, args) = matches.subcommand().unwrap();

    match task {
        "learning-ntsc-openfx-build-plugin" => {
            learning_ntsc_openfx_build_plugin::main(args).unwrap();
        }
        "learning-ofx-guide-1-build-plugin" => {
            learning_ofx_guide_1_build_plugin::main(args).unwrap();
        }
        "learning-ofx-guide-2-build-plugin" => {
            learning_ofx_guide_2_build_plugin::main(args).unwrap();
        }
        _ => {
            println!("Invalid xtask: {task}");
            process::exit(1);
        }
    }
}
