fn main() {
    println!("cargo:rerun-if-changed=../../assets/shell.rc");
    println!("cargo:rerun-if-changed=../../assets/fileflip.ico");
    embed_resource::compile("../../assets/shell.rc", embed_resource::NONE).manifest_optional().unwrap();
}
