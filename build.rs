fn main() {
    glib_build_tools::compile_resources(
        &["assets/resources"],
        "assets/resources/resources.gresource.xml",
        "pentas.gresource",
    );
}
