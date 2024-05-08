fn main() {
    glib_build_tools::compile_resources(
        &["src/resources/style"],
        "src/resources/style/resources.gresource.xml",
        "src.style.gresource",
    );
}
