use tempfile::TempDir;


#[test]
fn status_test() {
    let temp_dir = TempDir::new().expect("unable to create a temporary working directory");
    let file_1 = temp_dir.path().join("alice.txt");
    let file_2 = temp_dir.path().join("index");
}
