use anyhow::anyhow;
use anyhow::Result;
use gitclone::Command;
use std::path::Path;
use std::env::current_dir;

use clap::{App, Arg, SubCommand};

extern crate clap;
fn main() -> Result<()> {
    let matches = App::new("My git clone")
        .version("0.1")
        .subcommand(
            SubCommand::with_name("init").arg(
                Arg::with_name("PATH")
                    .help("the path where git should be initialized")
                    .required(false),
            ),
        )
        .subcommand(
            SubCommand::with_name("status").help("list untracked files")
            )
        .subcommand(
            App::new("commit")
                .arg(
                    Arg::from_usage("-m --message=[MESSAGE] 'Add the commit message'")
                        .required(true),
                )
                .arg(Arg::from_usage("--author=[author] 'The name of the author'").required(false))
                .arg(Arg::from_usage("--email=[email] 'The email of the author'").required(false)),
        )
        .subcommand(
            SubCommand::with_name("add").arg(
                Arg::with_name("FILE")
                    .help("the FILE to add into the index")
                    .required(false)
                    .multiple(true),
            ),
        )
        .get_matches();
    match matches.subcommand() {
        ("status", Some(_)) => {
            let root_path = current_dir();
            match root_path {
                Ok(root_path) => {
                    let command = Command::new(root_path)?;
                    command.status()
                }
                Err(e) => Err(anyhow!(e)),
            }
        }
        ("init", Some(_matches)) => {
            let root_path = _matches
                .value_of("PATH")
                .map_or(current_dir(), |v| Path::new(v).canonicalize());
            match root_path {
                Ok(root_path) => {
                    let command = Command::new(root_path)?;
                    command.init()
                }
                Err(e) => Err(anyhow!(e)),
            }
        }
        ("add", Some(_matches)) => {
            let inputs = _matches.values_of("FILE").unwrap();
            let paths = inputs.map(|v| Path::new(v).to_path_buf()).collect();
            let root_path = current_dir();
            match root_path {
                Ok(root_path) => {
                    let mut command = Command::new(root_path)?;
                    command.add(paths)
                }
                Err(e) => Err(anyhow!(e)),
            }
        }
        ("commit", Some(_matches)) => {
            let message = _matches
                .value_of("message")
                .expect("unable to get the message for the commit");
            let author = _matches
                .value_of("author")
                .or(option_env!("GIT_AUTHOR_NAME"))
                .expect("unable to get the author of the commit");
            let email = _matches
                .value_of("email")
                .or(option_env!("GIT_AUTHOR_EMAIL"))
                .expect("unable to get the email of author of the commit");
            let root_path = current_dir();
            match root_path {
                Ok(root_path) => {
                    let mut command = Command::new(root_path)?;
                    command.commit(author, email, message)
                }
                Err(e) => Err(anyhow!(e)),
            }
        }
        _ => unreachable!(),
    }
}
