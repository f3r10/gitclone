use anyhow::Result;
use anyhow::anyhow;
use chrono::Local;
use gitclone::{Author, Commit, EntryAdd, Index, Object, Refs, util};
use gitclone::{Database, Workspace};
use std::collections::HashMap;
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
                    .required(false)
                    .multiple(true),
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
            let inputs = _matches
                .values_of("FILE").unwrap();
            let paths = inputs.map(|v| Path::new(v).to_path_buf()).collect();
            let root_path = current_dir();
            match root_path {
                Ok(root_path) => {
                    let workspace = Workspace::new(&root_path);
                    let db = Database::new(&workspace.get_db_path());
                    let mut index = Index::new(workspace.get_git_path().join("index"));
                    if workspace.get_git_path().join("index").exists() {
                        index.load()?;
                    }
                    let tree = workspace.build_add_tree(paths, &db)?;
                    workspace.create_index_entry(&tree, &db, &mut index)?;
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
                    let database = Database::new(&workspace.get_db_path());
                    let mut index = Index::new(workspace.get_git_path().join("index"));
                    if workspace.get_git_path().join("index").exists() {
                        index.load()?;
                    } else {
                        return Err(anyhow!("Unable to commit if there is not a index file"));
                    }
                    let entries = index.each_entry()?;
                    // let entries_paths: Vec<_> = 
                    //     entries.into_iter().map(|e| e.path.to_path_buf()).collect();
                    // let tree = workspace.build_tree(entries_paths.clone(), &database)?;
                    let root_oid = util::build(entries, &database)?;
                    println!("root_oid: {:?}", root_oid);
                    // println!("t: {:?}", t)
                    // println!("tree: {:?}", tree);
                    // for e in  entries_paths {
                    //     println!("entries to commit: {:?}, dir {:?}, parent: {:?} ", e, e.is_dir(), e.parent());
                    // }
                    //
                    // let ( _, root_oid ) = workspace.build_root_tree(None, &database)?;
                    let refs = Refs::new(&workspace.get_git_path());
                    let parent = refs.read_head();
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
                        "[{} {:?}] {:?}",
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
