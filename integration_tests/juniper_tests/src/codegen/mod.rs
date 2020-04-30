mod derive_enum;
mod derive_input_object;
mod derive_object;
mod derive_object_with_raw_idents;
mod derive_union;
mod impl_scalar;
mod impl_union;
mod impl_object;
mod scalar_value_transparent;

use std::{
    fs::{read_dir, DirEntry},
    io,
    path::{Path, PathBuf},
};

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

// TODO: [Object] Type Validation: §4 (interfaces) for objects
// TODO: [Non-Null] §1 A Non‐Null type must not wrap another Non‐Null type.

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
