fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile proto files for gRPC services
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "proto/incidents.proto",
                "proto/alerts.proto",
                "proto/integrations.proto",
            ],
            &["proto"],
        )?;

    println!("cargo:rerun-if-changed=proto/");
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}
