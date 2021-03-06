use std::fs::{self, OpenOptions, Permissions};
use std::os::unix::prelude::PermissionsExt;
use std::{env, path::Path, process};
use assert_cmd::prelude::*;
use gitclone::util;
use predicates::str::{PredicateStrExt, is_empty, is_match};
use predicates::str::contains;

use tempfile::TempDir;


#[test]
fn list_untracked_files_in_name_order() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    // assert!(env::set_current_dir(&temp_dir).is_ok());
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec![(Path::new("file.txt").to_path_buf(), "".as_bytes()), (Path::new("another.txt").to_path_buf(), "".as_bytes())]).unwrap();
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
    // assert!(env::set_current_dir(&temp_dir).is_ok());
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec![(Path::new("commited.txt").to_path_buf(), "".as_bytes())]).unwrap();
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
        util::write_file(&temp_dir.path().to_owned(), vec![(Path::new("file.txt").to_path_buf(), "".as_bytes())]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^(\\?\\?) (file.txt\\n)$").unwrap().normalize());
}

#[test]
fn list_untracked_directories_not_their_contents() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec![(Path::new("file.txt").to_path_buf(), "".as_bytes()), (Path::new("dir").join("another.txt"), "".as_bytes())]).unwrap();
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
        .stdout(is_match("^(\\?\\? dir/\n\\?\\? file.txt\\n)$").unwrap());
}

#[test]
fn list_untracked_files_inside_tracked_directories() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec![(Path::new("a/b/").join("inner.txt"), "".as_bytes())]).unwrap();
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

    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec![(Path::new("a/").join("outer.txt"), "".as_bytes()), (Path::new("a/b/c/").join("file.txt"), "".as_bytes())]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^(\\?\\? a/b/c/\n\\?\\? a/outer.txt\\n)$").unwrap());
}

#[test]
fn does_not_list_empty_untracked_directories() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["init"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("Initialized empty Jit repository in"));
    std::fs::create_dir(temp_dir.path().join("outer")).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());
}

#[test]
fn list_untracked_directories_that_indirectly_contain_files() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    let _paths =
        util::write_file(&temp_dir.path().to_owned(), vec![(Path::new("outer/inner/").join("file.txt"), "".as_bytes())]).unwrap();
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
        .stdout(is_match("^(\\?\\? outer/\n)$").unwrap());
}

#[test]
fn prints_nothing_when_no_files_are_changed() {
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let _paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());
}

#[test]
fn reports_files_with_modified_contents() {
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let _paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    util::write_file(&temp_path, 
        vec![ (Path::new("1.txt").to_path_buf(), "changed".as_bytes()),
        (Path::new("a").join("2.txt"), "modified".as_bytes()),
        ]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( M 1.txt\\n M a/2.txt\\n)$").unwrap());
}

#[test]
fn reports_files_with_changed_modes() {
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();

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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));

    let perms = Permissions::from_mode(0o100755);
    fs::set_permissions(paths[1].to_path_buf(), perms).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( M a/2.txt\\n)$").unwrap());
}


#[test]
fn reports_modified_files_with_unchanged_size() {
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let _paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    util::write_file(&temp_path, 
        vec![(Path::new("a/b/").join("3.txt"), "hello".as_bytes())]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( M a/b/3.txt\\n)$").unwrap());
}


#[test]
fn prints_nothing_if_a_file_is_touched() {
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let _paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    let _f = OpenOptions::new().write(true).truncate(false).open("1.txt").unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());
}

#[test]
fn reports_deleted_files() {
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let _paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    fs::remove_file("a/2.txt").unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( D a/2.txt\\n)$").unwrap());
}


#[test]
fn reports_files_in_deleted_directories() {
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let _paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    fs::remove_dir_all("a").unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( D a/2.txt\\n D a/b/3.txt\\n)$").unwrap());
}



#[test]
fn reports_a_file_added_to_a_tracked_directory() {
    // before do
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let _paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    //end
    let _paths =
        util::write_file(&temp_path, 
            vec![ 
            (Path::new("a").join("4.txt"), "four".as_bytes()),
            ]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["add", "."])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());

    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( A a/4.txt\\n)$").unwrap());
}


#[test]
fn reports_a_file_added_to_an_untracked_directory() {
    // before do
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let _paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    //end
    let _paths =
        util::write_file(&temp_path, 
            vec![ 
            (Path::new("d/e/").join("5.txt"), "five".as_bytes()),
            ]).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["add", "."])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());

    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( A d/e/5.txt\\n)$").unwrap());
}


#[test]
fn reports_modified_modes() {
    // before do
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    //end
    let perms = Permissions::from_mode(0o100755);
    fs::set_permissions(paths[0].to_path_buf(), perms).unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["add", "."])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());

    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( M 1.txt\\n)$").unwrap());
}


#[test]
fn reports_deleted_files_head_index() {
    // before do
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    //end
    fs::remove_file(paths[0].to_path_buf()).unwrap();
    fs::remove_file(".git/index").unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["add", "."])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());

    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( D 1.txt\\n)$").unwrap());
}


#[test]
fn reports_all_deleted_files_inside_directories() {
    // before do
    let temp_dir = TempDir::new().unwrap();
    assert!(env::set_current_dir(&temp_dir).is_ok());
    let temp_path = temp_dir.path().to_owned();
    let paths =
        util::write_file(&temp_path, 
            vec![ (Path::new("1.txt").to_path_buf(), "one".as_bytes()),
            (Path::new("a").join("2.txt"), "two".as_bytes()),
            (Path::new("a/b/").join("3.txt"), "three".as_bytes())
            ]).unwrap();
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
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["commit", "-m", "commit message"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("commit message"));
    //end
    fs::remove_dir_all("a").unwrap();
    fs::remove_file(".git/index").unwrap();
    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["add", "."])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_empty());

    process::Command::cargo_bin("git-clone")
        .unwrap()
        .args(&["status"])
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(is_match("^( D a/2.txt\\n D a/b/3.txt\\n)$").unwrap());
}
