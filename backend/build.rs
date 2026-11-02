fn main() {
    // SQLx embeds migrations at compile time. Renames/additions must invalidate
    // the build as well as edits to migrations that already existed.
    println!("cargo:rerun-if-changed=../migrations");
}
