fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var_os("CARGO_CFG_WINDOWS").is_none()
        || std::env::var("PROFILE").as_deref() != Ok("release")
    {
        return;
    }

    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        embed_manifest_with_windres();
    } else {
        let mut resource = winresource::WindowsResource::new();
        resource.set_manifest_file("app.manifest");
        resource
            .compile()
            .expect("failed to embed the Windows application manifest");
    }
}

fn embed_manifest_with_windres() {
    use std::{fs, process::Command};

    // GNU windres invokes its preprocessor with an unquoted working path. Keep
    // generated resource inputs in a stable no-space temporary directory.
    let resource_dir = std::env::temp_dir().join("dji_mic_mapper_resource");
    fs::create_dir_all(&resource_dir).expect("failed to create resource directory");
    let manifest = resource_dir.join("app.manifest");
    let resource_script = resource_dir.join("app.rc");
    let resource_object = resource_dir.join("app.o");
    fs::copy("app.manifest", &manifest).expect("failed to stage app.manifest");
    fs::write(&resource_script, "1 24 \"app.manifest\"\n")
        .expect("failed to create resource script");

    let status = Command::new("windres")
        .current_dir(&resource_dir)
        .arg("--input")
        .arg(&resource_script)
        .arg("--output-format=coff")
        .arg("--output")
        .arg(&resource_object)
        .status()
        .expect("failed to start windres");
    assert!(status.success(), "windres failed to embed app.manifest");
    println!("cargo:rustc-link-arg={}", resource_object.display());
}
