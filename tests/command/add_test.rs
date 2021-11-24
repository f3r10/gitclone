use std::env::current_dir;
use std::{env, os::unix::prelude::PermissionsExt, process};
use std::fs::Permissions;
use std::fs;
use predicates::str::{contains, is_empty};
use assert_cmd::prelude::*;

use gitclone::{util, Command};
use tempfile::{TempDir, tempdir};

#[test]
fn cli_add_without_init() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    assert!(env::set_current_dir(&temp_dir).is_ok());
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["add", "hello.txt"])
        .current_dir(&temp_dir)
        .assert()
        .stderr(contains("Error: not a git repository (or any parent up to mount point /)"));
}
#[test]
fn cli_add() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec!["hello.txt", "hello"]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["init"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("Initialized empty Jit repository in"));
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["add", "hello.txt"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());
    assert!(temp_dir.path().join(".git/index").exists())
}

#[test]
fn adds_regular_file_to_index() {
    let temp_dir = tempdir().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let paths =
        util::write_file(&temp_dir.path().to_owned(), vec!["hello.txt", "hello"]).unwrap();
    let mut command = Command::new(temp_dir.path().to_path_buf()).unwrap();
    command.init().unwrap();
    command.add(paths[..1].to_vec()).unwrap();
    let entries = command
        .index
        .each_entry()
        .unwrap()
        .iter()
        .flat_map(|e| vec![(e.get_mode().unwrap(), e.get_name())])
        .collect::<Vec<_>>();
    assert_eq!(entries, vec![(0o100644, "hello.txt".to_string())])
}


#[test]
fn adds_an_executable_file_to_index() {
    let temp_dir = TempDir::new().unwrap();
    println!("temp_dir: {:?}", temp_dir);
    assert!(env::set_current_dir(&temp_dir).is_ok());
    println!("current dir 1 {:?}", current_dir());
    let temp_path = temp_dir.path().to_owned();
    let paths =
        util::write_file(&temp_dir.path().to_owned(), vec!["hello.txt", "hello"]).unwrap();
    println!("current dir 2 {:?}", current_dir());
    let perms = Permissions::from_mode(0o100755);
    fs::set_permissions(paths[0].to_path_buf(), perms).unwrap();
    let mut command = Command::new(temp_path).unwrap();
    command.init().unwrap();
    command.add(paths[..1].to_vec()).unwrap();
    let entries = command
        .index
        .each_entry()
        .unwrap()
        .iter()
        .flat_map(|e| vec![(e.get_mode().unwrap(), e.get_name())])
        .collect::<Vec<_>>();
    assert_eq!(entries, vec![(0o100755, "hello.txt".to_string())])
}