fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=migrations");
    tonic_build::configure().compile(&["proto/hello.proto"], &["proto"])?;
    Ok(())
}
