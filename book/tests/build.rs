fn main() {
    let files = skeptic::markdown_files_of_directory("../src/");
    skeptic::generate_doc_tests(&files);
}
