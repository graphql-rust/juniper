extern crate skeptic;

fn main() {
    let files = skeptic::markdown_files_of_directory("../content/");
    skeptic::generate_doc_tests(&files);
}
