extern crate embed_resource;

fn main() {

    let author = env!("CARGO_PKG_AUTHORS").replace( "<", "\\<" ).replace( ">", "\\>" );

    let macros = [
        format!( "REPLACE_VERSION_COMMA={},{},{},0",
            env!("CARGO_PKG_VERSION_MAJOR"),
            env!("CARGO_PKG_VERSION_MINOR"),
            env!("CARGO_PKG_VERSION_PATCH")
        ),
        format!( "REPLACE_ORIGINALFILENAME=\\\"{}.exe\\\"", env!("CARGO_PKG_NAME") ),
        format!( "REPLACE_FILEDESCRIPTION=\\\"{}\\\"", env!("CARGO_PKG_NAME") ),
        format!( "REPLACE_PRODUCTNAME=\\\"{}\\\"", env!("CARGO_PKG_NAME") ),
        format!( "REPLACE_PRODUCTVERSION=\\\"Ver. {}\\\"", env!("CARGO_PKG_VERSION")),
        format!( "REPLACE_LEGALCOPYRIGHT=\\\"Copyright (C) {}. All rights reserved.\\\"", author )
    ];

    dbg!( &macros );

    embed_resource::compile("versioninfo.rc", &macros )
        .manifest_optional()
        .unwrap();
}