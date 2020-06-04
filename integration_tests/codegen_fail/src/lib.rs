// TODO: [Object] Type Validation: §4 (interfaces) for objects
// TODO: [Non-Null] §1 A Non‐Null type must not wrap another Non‐Null type.

#[cfg(test)]
use std::{
    fs::{read_dir, DirEntry},
    io,
    path::{Path, PathBuf},
};

#[cfg(test)]
fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}

#[test]
fn test_failing_compiliation() {
    let t = trybuild::TestCases::new();
    let dir = PathBuf::from("fail");

    visit_dirs(dir.as_path(), &|entry: &DirEntry| {
        if let Some(Some("rs")) = entry.path().extension().map(|os| os.to_str()) {
            t.compile_fail(entry.path());
        }
    })
    .unwrap();
}
