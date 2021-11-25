use std::{env, path::Path, process};
use assert_cmd::prelude::*;
use gitclone::util;
use predicates::str::{contains, is_empty, starts_with};

use tempfile::TempDir;


#[test]
fn commit_test() {
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
}
