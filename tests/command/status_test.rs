use std::{env, path::Path, process};
use assert_cmd::prelude::*;
use gitclone::util;
use predicates::str::{PredicateStrExt, is_empty, is_match};
use predicates::str::contains;

use tempfile::TempDir;


#[test]
fn list_untracked_files_in_name_order() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec![Path::new("file.txt").to_path_buf(), Path::new("another.txt").to_path_buf()]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["init"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("Initialized empty Jit repository in"));
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains(
                "\
                ?? another.txt\n\
                ?? file.txt\
                "
                ).count(1));
}

#[test]
fn list_files_as_untracked_if_they_are_not_in_index() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec![Path::new("commited.txt").to_path_buf()]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["init"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("Initialized empty Jit repository in"));
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["add", "."])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());
    assert!(temp_dir.path().join(".git/index").exists());
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    let _untracked_paths =
        util::write_file(&temp_dir.path().to_owned(), vec![Path::new("file.txt").to_path_buf()]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^(\\?\\?) (file.txt\\n)$").unwrap().normalize());
}
