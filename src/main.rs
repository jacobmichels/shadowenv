mod cli;
mod diff;
mod execcmd;
mod features;
mod hash;
mod hook;
mod init;
mod lang;
mod loader;
mod output;
mod prompt_widget;
mod shadowenv;
mod trust;
mod undo;

use crate::{hook::VariableOutputMode, shadowenv::Shadowenv};
use anyhow::{anyhow, Error};
use std::{env, path::PathBuf, process};

fn main() {
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(_) => return, // If the current dir was deleted, there's not much we can do. Just exit silently.
    };

    match cli::app().get_matches().subcommand() {
        ("hook", Some(matches)) => {
            let legacy_fallback_data = matches.value_of("$__shadowenv_data").map(|d| d.to_string());
            let data = Shadowenv::load_shadowenv_data_or_legacy_fallback(legacy_fallback_data);
            let shellpid = determine_shellpid_or_crash(matches.value_of("shellpid"));
            let force = matches.is_present("force");

            let mode = match true {
                true if matches.is_present("porcelain") => VariableOutputMode::Porcelain,
                true if matches.is_present("fish") => VariableOutputMode::Fish,
                true if matches.is_present("json") => VariableOutputMode::Json,
                true if matches.is_present("pretty-json") => VariableOutputMode::PrettyJson,
                _ => VariableOutputMode::Posix,
            };
            if let Err(err) = hook::run(current_dir, data, mode, force) {
                process::exit(output::handle_hook_error(
                    err,
                    shellpid,
                    matches.is_present("silent"),
                ));
            }
        }
        ("diff", Some(matches)) => {
            let verbose = matches.is_present("verbose");
            let color = !matches.is_present("no-color");
            let legacy_fallback_data = matches.value_of("$__shadowenv_data").map(|d| d.to_string());
            let data = Shadowenv::load_shadowenv_data_or_legacy_fallback(legacy_fallback_data);
            process::exit(diff::run(verbose, color, data));
        }
        ("prompt-widget", Some(matches)) => {
            let legacy_fallback_data = matches.value_of("$__shadowenv_data").map(|d| d.to_string());
            let data = Shadowenv::load_shadowenv_data_or_legacy_fallback(legacy_fallback_data);
            process::exit(prompt_widget::run(data));
        }
        ("trust", Some(matches)) => {
            let dir = matches
                .value_of("dir")
                .map(PathBuf::from)
                .unwrap_or(current_dir);

            if let Err(err) = trust::run(dir) {
                eprintln!("{}", err); // TODO: better formatting
                process::exit(1);
            }
        }
        ("exec", Some(matches)) => {
            let legacy_fallback_data = matches.value_of("$__shadowenv_data").map(|d| d.to_string());
            let data = Shadowenv::load_shadowenv_data_or_legacy_fallback(legacy_fallback_data);
            let argv: Vec<&str> = match (
                matches.value_of("child-argv0"),
                matches.values_of("child-argv"),
            ) {
                (_, Some(argv)) => argv.collect(),
                (Some(argv0), _) => vec![argv0],
                (_, _) => unreachable!(),
            };
            let dir = matches.value_of("dir");
            let pathbuf = dir.map(PathBuf::from).unwrap_or(current_dir);
            if let Err(err) = execcmd::run(pathbuf, data, argv) {
                eprintln!("{}", err);
                process::exit(1);
            }
        }
        ("init", Some(matches)) => {
            let shellname = matches.subcommand_name().unwrap();
            process::exit(init::run(shellname));
        }
        _ => {
            panic!("subcommand was required by config but somehow none was provided");
        }
    }
}

fn determine_shellpid_or_crash(arg: Option<&str>) -> u32 {
    match arg {
        Some(arg) => arg
            .parse::<u32>()
            .expect("shadowenv error: invalid non-numeric argument for --shellpid"),
        None => unsafe_getppid().expect("shadowenv bug: unable to get parent pid"),
    }
}

fn unsafe_getppid() -> Result<u32, Error> {
    let ppid;
    unsafe { ppid = libc::getppid() }
    if ppid < 1 {
        return Err(anyhow!("somehow failed to get ppid"));
    }
    Ok(ppid as u32)
}
