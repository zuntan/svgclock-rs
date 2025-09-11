extern crate embed_resource;

fn main() {
    let macros = &[
        format!( "REPLACE_NAME=\"{}\"", env!("CARGO_PKG_NAME") ),
        format!( "REPLACE_VERSION_RAW_WITH_VAR=\"Ver. {}\"", env!("CARGO_PKG_VERSION")),
        format!( "REPLACE_VERSION_COMMA={},{},{},0",
            env!("CARGO_PKG_VERSION_MAJOR"),
            env!("CARGO_PKG_VERSION_MINOR"),
            env!("CARGO_PKG_VERSION_PATCH")
        ),
        format!( "REPLACE_AUTHOR=\"{}\"", env!("CARGO_PKG_AUTHORS") )
    ];

    dbg!( macros );

    embed_resource::compile("versioninfo.rc", macros )
        .manifest_optional()
        .unwrap();
}