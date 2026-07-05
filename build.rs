fn main() {
    println!("cargo:rerun-if-changed=proto/norion/runtime/v1/runtime.proto");

    #[cfg(feature = "runtime-tonic")]
    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&["proto/norion/runtime/v1/runtime.proto"], &["proto"])
        .expect("compile norion runtime tonic proto");
}
