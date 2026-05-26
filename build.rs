fn main() {
    cc::Build::new()
        .file("src/arch/x86_64/context.S")
        .flag("-ffreestanding")
        .flag("-fno-stack-protector")
        .flag("-mno-red-zone")
        .flag("-nostdlib")
        .compile("context");

    println!("cargo:rerun-if-changed=src/arch/x86_64/context.S");
}
