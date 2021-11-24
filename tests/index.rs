use std::fs;

use anyhow::Result;
use gitclone::{util, Index};
use ring::{
    digest,
    rand::{SecureRandom, SystemRandom},
};
use tempfile::TempDir;

// Should add a single file to the index file
#[test]
fn add_a_single_file() -> Result<()> {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    let file_to_add = temp_dir.path().join("alice.txt");
    let index_file = temp_dir.path().join("index");
    fs::create_dir_all(&file_to_add)?;
    fs::create_dir_all(&index_file)?;
    let rng = SystemRandom::new();
    let mut oid = [0u8; digest::SHA1_OUTPUT_LEN];
    rng.fill(&mut oid).unwrap();
    let stat = util::stat_file(file_to_add.to_path_buf())?;
    let mut index = Index::new(&index_file);
    index.add(file_to_add.to_path_buf(), oid.to_vec(), stat)?;
    let entries = index
        .each_entry()?
        .iter()
        .map(move |e| e.path.to_path_buf())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec![file_to_add]);
    Ok(())
}

// Should replace a file with a directory
#[test]
fn replace_a_file_with_directory() -> Result<()> {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    let file_to_add_1 = temp_dir.path().join("alice.txt");
    let file_to_add_2 = temp_dir.path().join("bob.txt");
    let dir = temp_dir.path().join("alice.txt/nested.txt");
    let index_file = temp_dir.path().join("index");

    fs::create_dir_all(&file_to_add_1)?;
    fs::create_dir_all(&file_to_add_2)?;
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(&index_file)?;

    let rng = SystemRandom::new();
    let mut oid = [0u8; digest::SHA1_OUTPUT_LEN];
    rng.fill(&mut oid).unwrap();

    let stat_1 = util::stat_file(file_to_add_1.to_path_buf())?;
    let stat_2 = util::stat_file(file_to_add_2.to_path_buf())?;
    let stat_3 = util::stat_file(dir.to_path_buf())?;

    let mut index = Index::new(&index_file);
    index.add(file_to_add_1.to_path_buf(), oid.to_vec(), stat_1)?;
    index.add(file_to_add_2.to_path_buf(), oid.to_vec(), stat_2)?;
    index.add(dir.to_path_buf(), oid.to_vec(), stat_3)?;

    let entries = index
        .each_entry()?
        .iter()
        .map(move |e| e.path.to_path_buf())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec![dir, file_to_add_2]);
    Ok(())
}

// Should replace a directory with a file
#[test]
fn replace_dir_with_file() -> Result<()> {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    let file_to_add_1 = temp_dir.path().join("alice.txt");
    let file_to_add_2 = temp_dir.path().join("nested/bob.txt");
    let dir = temp_dir.path().join("nested");
    let index_file = temp_dir.path().join("index");

    fs::create_dir_all(&file_to_add_1)?;
    fs::create_dir_all(&file_to_add_2)?;
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(&index_file)?;

    let rng = SystemRandom::new();
    let mut oid = [0u8; digest::SHA1_OUTPUT_LEN];
    rng.fill(&mut oid).unwrap();

    let stat_1 = util::stat_file(file_to_add_1.to_path_buf())?;
    let stat_2 = util::stat_file(file_to_add_2.to_path_buf())?;
    let stat_3 = util::stat_file(dir.to_path_buf())?;

    let mut index = Index::new(&index_file);
    index.add(file_to_add_1.to_path_buf(), oid.to_vec(), stat_1)?;
    index.add(file_to_add_2.to_path_buf(), oid.to_vec(), stat_2)?;
    index.add(dir.to_path_buf(), oid.to_vec(), stat_3)?;

    let entries = index
        .each_entry()?
        .iter()
        .map(move |e| e.path.to_path_buf())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec![file_to_add_1, dir]);
    Ok(())
}

// should recursively replaces a directory with a file
#[test]
fn replace_recursively_dir_with_file() -> Result<()> {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    let file_to_add_1 = temp_dir.path().join("alice.txt");
    let file_to_add_2 = temp_dir.path().join("nested/bob.txt");
    let file_to_add_3 = temp_dir.path().join("nested/inner/claire.txt");
    let dir = temp_dir.path().join("nested");
    let index_file = temp_dir.path().join("index");

    fs::create_dir_all(&file_to_add_1)?;
    fs::create_dir_all(&file_to_add_2)?;
    fs::create_dir_all(&file_to_add_3)?;
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(&index_file)?;

    let rng = SystemRandom::new();
    let mut oid = [0u8; digest::SHA1_OUTPUT_LEN];
    rng.fill(&mut oid).unwrap();

    let stat_1 = util::stat_file(file_to_add_1.to_path_buf())?;
    let stat_2 = util::stat_file(file_to_add_2.to_path_buf())?;
    let stat_3 = util::stat_file(file_to_add_3.to_path_buf())?;
    let stat_4 = util::stat_file(dir.to_path_buf())?;

    let mut index = Index::new(&index_file);
    index.add(file_to_add_1.to_path_buf(), oid.to_vec(), stat_1)?;
    index.add(file_to_add_2.to_path_buf(), oid.to_vec(), stat_2)?;
    index.add(file_to_add_3.to_path_buf(), oid.to_vec(), stat_3)?;
    index.add(dir.to_path_buf(), oid.to_vec(), stat_4)?;

    let entries = index
        .each_entry()?
        .iter()
        .map(move |e| e.path.to_path_buf())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec![file_to_add_1, dir]);
    Ok(())
}
