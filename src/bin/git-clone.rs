use anyhow::Result;
use chrono::Local;
use gitclone::{Author, Blob, Commit, Index, Object, Refs, util};
use gitclone::{Database, Workspace};
use std::path::Path;
use std::{env::current_dir, fs};

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
                    .required(false),
            ),
        )
        .get_matches();
    match matches.subcommand() {
        ("init", Some(_matches)) => {
            let root_path = _matches
                .value_of("PATH")
                .map_or(current_dir(), |v| Path::new(v).canonicalize());
            match root_path {
                Ok(root_path) => {
                    let git_path = root_path.join(".git/");
                    for dir in ["objects", "refs"] {
                        fs::create_dir_all(git_path.join(dir)).expect("unable to create path")
                    }
                    println!(
                        "Initialized empty Jit repository in: {:?}",
                        git_path.to_str()
                    );
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        ("add", Some(_matches)) => {
            let path = _matches
                .value_of("FILE")
                .map(|v| Path::new(v)).expect("it is necessary to add a file");
            let root_path = current_dir();
            match root_path {
                Ok(root_path) => {
                    let workspace = Workspace::new(&root_path);
                    let database = Database::new(&workspace.get_db_path());
                    let mut index = Index::new(workspace.get_git_path().join("index"));
                    let stat = util::stat_file(path.clone().canonicalize()?)?;
                    let blob = Blob::new(path.clone().canonicalize()?)?;
                    database.store(&blob)?;
                    index.add(path.clone().to_path_buf(), blob.get_oid().to_string(), stat)?;
                    index.write_updates()?;
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
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
                    let workspace = Workspace::new(&root_path);
                    let ( tree, root_oid ) = workspace.build_root_tree()?;
                    let database = Database::new(&workspace.get_db_path());
                    let refs = Refs::new(&workspace.get_git_path());
                    let parent = refs.read_head();
                    workspace.persist_tree(&tree, &database)?;
                    let current_time = Local::now();
                    let author = Author::new(author, email, current_time);
                    let commit = Commit::new(
                        root_oid.to_string(),
                        author,
                        message.to_string(),
                        parent.clone(),
                    );
                    database.store(&commit)?;
                    refs.update_head(commit.get_oid().to_string())?;
                    let is_root = if parent.is_none() {
                        "(root-commit)"
                    } else {
                        ""
                    };
                    println!(
                        "[{} {}] {:?}",
                        is_root,
                        &commit.get_oid(),
                        message.lines().next()
                    )
                }
                Err(e) => {
                    println!("Error: {}", e)
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
