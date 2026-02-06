fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo to rerun if proto changes
    println!("cargo:rerun-if-changed=proto/job_orchestrator.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&["proto/job_orchestrator.proto"], &["proto/"])?;

    Ok(())
}
