// Avoid spawning an console window for the program.
// This is ignored on other platforms.
// https://msdn.microsoft.com/en-us/library/4cc7ya5b.aspx for more information.
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_imports)]

mod cli;
mod gui;
mod localization;
#[cfg(target_os = "windows")]
mod process;
#[cfg(target_os = "windows")]
mod tray;

#[macro_use]
extern crate log;

use grin_gui_core::config::Config;
use grin_gui_core::fs::{PersistentData, CONFIG_DIR};
use grin_gui_core::utility::{remove_file, rename};
use grin_gui_core::{logger, LoggingConfig};

#[cfg(target_os = "linux")]
use anyhow::Context;
use std::env;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

pub fn main() {
	let opts_result = cli::get_opts();

	#[cfg(debug_assertions)]
	let is_debug = true;
	#[cfg(not(debug_assertions))]
	let is_debug = false;

	// If this is a clap error, we map to None since we are going to exit and display
	// an error message anyway and this value won't matter. If it's not an error,
	// the underlying `command` will drive this variable. If a `command` is passed
	// on the command line, Grin GUI functions as a CLI instead of launching the GUI.
	/*let is_cli = opts_result
	.as_ref()
	.map(|o| &o.command)
	.unwrap_or(&None)
	.is_some();*/

	// disable command line for now
	let is_cli = false;

	// This function validates whether or not we need to exit and print any message
	// due to arguments passed on the command line. If not, it will return a
	// parsed `Opts` struct. This also handles setting up our windows release build
	// fix that allows us to print to the console when not using the GUI.
	let opts = cli::validate_opts_or_exit(opts_result, is_cli, is_debug);

	let config_dir_local = {
		let mut config_dir = CONFIG_DIR.lock().unwrap();
		if let Some(data_dir) = &opts.data_directory {
			*config_dir = data_dir.clone();
		}
		config_dir.clone()
	};

	// Set up logging config for grin-gui code itself
	let mut gui_logging_config = LoggingConfig::default();
	gui_logging_config.tui_running = Some(false);
	gui_logging_config.stdout_log_level = log::Level::Debug;
	gui_logging_config.file_log_level = log::Level::Debug;
	let mut gui_log_dir = config_dir_local.clone();
	gui_log_dir.push("grin-gui.log");
	gui_logging_config.log_file_path = gui_log_dir.into_os_string().into_string().unwrap();

	logger::update_logging_config(logger::LogArea::Gui, gui_logging_config);

	// Called when we launch from the temp (new release) binary during the self update
	// process. We will rename the temp file (running process) to the original binary
	if let Some(cleanup_path) = &opts.self_update_temp {
		if let Err(e) = handle_self_update_temp(cleanup_path) {
			log_error(&e);
			std::process::exit(1);
		}
	}

	log_panics::init();

	log::info!("Grin GUI {} has started.", VERSION);

	// Ensures another instance of Grin GUI isn't already running.
	#[cfg(target_os = "windows")]
	process::avoid_multiple_instances();

	/*match opts.command {
	Some(command) => {
		// Process the command and exit
		if let Err(e) = match command {
			cli::Command::Backup {
				backup_folder,
				destination,
				flavors,
				compression_format,
				level,
			} => command::backup(
				backup_folder,
				destination,
				flavors,
				compression_format,
				level,
			),
			cli::Command::Update => command::update_both(),
			cli::Command::UpdateAddons => command::update_all_addons(),
			cli::Command::UpdateAuras => command::update_all_auras(),
			cli::Command::Install { url, flavor } => command::install_from_source(url, flavor),
			cli::Command::PathAdd { path, flavor } => command::path_add(path, flavor),
		} {
			log_error(&e);
		}
	}
	None => {*/
	let config: Config = Config::load_or_default().expect("loading config on application startup");

	#[cfg(target_os = "windows")]
	tray::spawn_sys_tray(config.close_to_tray, config.start_closed_to_tray);

	// Start the GUI
	gui::run(opts, config);
	/*
	}*/
}

/// Log any errors
pub fn log_error(error: &anyhow::Error) {
	log::error!("{}", error);

	let mut causes = error.chain();
	// Remove first entry since it's same as top level error
	causes.next();

	for cause in causes {
		log::error!("caused by: {}", cause);
	}
}

pub fn error_cause_string(error: &anyhow::Error) -> String {
	let mut ret_val = String::new();
	let mut causes = error.chain();

	// Remove first entry since it's same as top level error
	let top_level_cause = causes.next();
	if let Some(t) = top_level_cause {
		ret_val.push_str(&format!("{}\n\n", t));
	}

	for cause in causes {
		ret_val.push_str(&format!("{}\n\n", cause));
	}
	ret_val
}

fn handle_self_update_temp(cleanup_path: &Path) -> Result<()> {
	#[cfg(not(target_os = "linux"))]
	let current_bin = env::current_exe()?;

	#[cfg(target_os = "linux")]
	let current_bin =
		PathBuf::from(env::var("APPIMAGE").context("error getting APPIMAGE env variable")?);

	// Fix for self updating pre 0.5.4 to >= 0.5.4
	//
	// Pre 0.5.4, `cleanup_path` is actually the file name of the main bin name that
	// got passed via the CLI in the self update process. We want to rename the
	// current bin to that bin name. This was passed as a string of just the file
	// name, so we want to make an actual full path out of it first.
	if current_bin
		.file_name()
		.unwrap_or_default()
		.to_str()
		.unwrap_or_default()
		.starts_with("tmp_")
	{
		let main_bin_name = cleanup_path;

		let parent_dir = current_bin.parent().unwrap();

		let main_bin = parent_dir.join(&main_bin_name);

		rename(&current_bin, &main_bin)?;
	} else {
		remove_file(cleanup_path)?;
	}

	log::debug!("Grin GUI updated successfully");

	Ok(())
}
