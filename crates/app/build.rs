fn main() {
    println!("cargo:rerun-if-changed=../../assets/app.rc");
    println!("cargo:rerun-if-changed=../../assets/app.manifest");
    println!("cargo:rerun-if-changed=../../assets/fileflip.ico");
    embed_resource::compile("../../assets/app.rc", embed_resource::NONE).manifest_optional().unwrap();
}
