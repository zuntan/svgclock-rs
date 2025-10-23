mod minitemplate_tests {

    use svgclock_rs::minitemplate::*;
    use std::rc::Rc;
    use std::collections::HashSet;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test00() {
        let raw_text = "
            |This line is indented.
            |   Whitespace within indentation is preserved.
            |Last line.
            ";

        let trimmed_text = trim_margin(raw_text, "|");

        assert_eq!(trimmed_text, "This line is indented.\n   Whitespace within indentation is preserved.\nLast line.\n");
    }

    #[test]
    fn test01() {
        let source = "";
        let x  = parse( source.as_bytes() );
        assert!( x.is_ok() );

        let source = "ABC\nDEF";
        let x  = parse( source.as_bytes() );
        assert!( x.is_ok() );
    }

    #[test]
    fn test02() {
        let t  = parse_str("ABC\nDEF").unwrap();
        let mut c = Context::new();
        let s = t.render( &mut c );
        assert_eq!( s, "ABC\nDEF" );
    }

    #[test]
    fn test03() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAA
            |BBBBB
            |CCCCC
            |D {{ var }} D
            |EEEEE
            ", "|").as_str()
        ).unwrap();

        let mut c = Context::new();

        assert_eq!(
            trim_margin(
            "
            |AAAAA
            |BBBBB
            |CCCCC
            |D ???var??? D
            |EEEEE
            ", "|").as_str(), t.render( &mut c ) );

        c.set("var", "DDD" );

        assert_eq!(
            trim_margin(
            "
            |AAAAA
            |BBBBB
            |CCCCC
            |D DDD D
            |EEEEE
            ", "|").as_str(), t.render( &mut c ) );

        c.set( "var", "D< >D");

        assert_eq!(
            trim_margin(
            "
            |AAAAA
            |BBBBB
            |CCCCC
            |D D&lt;&nbsp;&gt;D D
            |EEEEE
            ", "|").as_str(), t.render( &mut c ) );

    }

    #[test]
    fn test04() {
        let mut c = Context::new();
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% if cond %}
            |BB {{ var1 }} BB
            |{% else %}
            |CC {{ var2 }} CC
            |{% end %}
            |DDDDDD
            ", "|").as_str()
        ).unwrap();

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |CC ???var2??? CC
            |
            |DDDDDD
            ", "|").as_str(), t.render( &mut c ) );

        c.is_def_blank = true;

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |CC  CC
            |
            |DDDDDD
            ", "|").as_str(), t.render( &mut c ) );

        c.set( "var2", "cc" );

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |CC cc CC
            |
            |DDDDDD
            ", "|").as_str(), t.render( &mut c ) );

        c.set( "cond", true );
        c.set( "var1", "bb" );

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |BB bb BB
            |
            |DDDDDD
            ", "|").as_str(), t.render( &mut c ) );

    }

    #[test]
    fn test05() {
        let mut c = Context::new();
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% if cond1 %}
            |{% if cond2 %}
            |BB {{ var2 }} BB
            |{% end %}
            |{% else %}
            |CC {{ var1e }} CC
            |{% if cond3 %}
            |DD {{ var3 }} DD
            |{% end %}
            |DD {{ var1e }} DD
            |{% end %}
            |EEEEEE
            ", "|").as_str()
        ).unwrap();

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |CC ???var1e??? CC
            |
            |DD ???var1e??? DD
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

        c.set( "cond1", true );

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

        c.set( "cond2", true );

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |
            |BB ???var2??? BB
            |
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

    }

    #[test]
    fn test06() {
        let mut c = Context::new();
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% for var in vars %}
            |BB {{ var.var1 }} BB {{ var.var2 }} BB
            |{% end %}
            |EEEEEE
            ", "|").as_str()
        ).unwrap();

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

        let mut vec: VecValue = Vec::new();

        vec.set(
            make_map_value(
                &[
                    ( "var1", ContextValue::from( "V1_1" ) ),
                    ( "var2", ContextValue::from( "V2_1" ) )
                    ]
                )
            );
        vec.set(
            make_map_value(
                &[
                    ( "var1", "V1_2".into() ),
                    ( "var2", "V2_2".into() )
                    ]
                )
            );

        c.set( "vars", vec );

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |BB V1_1 BB V2_1 BB
            |
            |BB V1_2 BB V2_2 BB
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

    }

    #[test]
    fn test07() {
        let mut c = Context::new();
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% if cond %}
            |{% for var in vars %}
            |BB {{ var }} BB
            |{% end %}
            |{% end %}
            |EEEEEE
            ", "|").as_str()
        ).unwrap();

        c.set( "vars", make_vec_value( &[
                ContextValue::from( 1 ),
                ContextValue::from( 2 ),
                ContextValue::from( 3 ),
                ContextValue::from( 4 ),
            ] ) );

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

        c.set( "cond", "true" );

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |
            |BB 1 BB
            |
            |BB 2 BB
            |
            |BB 3 BB
            |
            |BB 4 BB
            |
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

    }

    #[test]
    fn test08() {
        let mut c = Context::new();
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% if cond %}
            |{% for y in vars_y %}
            |{% for x in vars_x %}
            |BB Y{{ y }} X{{ x }} BB
            |{% end %}
            |{% end %}
            |{% end %}
            |EEEEEE
            ", "|").as_str()
        ).unwrap();

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

        c.set( "cond", true );

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

        c.set( "vars_y",  make_vec_value( &[ 1.into(), 2.into() ] ) );
        c.set( "vars_x",  make_vec_value( &[ 3.into(), 4.into() ] ) );

        assert_eq!(
            trim_margin(
            "
                |AAAAAA
                |
                |
                |
                |BB Y1 X3 BB
                |
                |BB Y1 X4 BB
                |
                |
                |
                |BB Y2 X3 BB
                |
                |BB Y2 X4 BB
                |
                |
                |
                |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );

    }

    #[test]
    fn test09() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% if cond  }}
            |EEEEEE
            ", "|").as_str()
        );

        assert!( t.is_err() );
        assert_eq!( t.unwrap_err().to_string(), "Parse error at Line:2 Col:12. `}}`" );
    }

    #[test]
    fn test10() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% if cond %}
            |EEEEEE
            ", "|").as_str()
        );

        assert!( t.is_err() );
        assert_eq!( t.unwrap_err().to_string(), "Parse error at end of text. Missing `{% end %}` /From Line:2 `{% if`" );
    }

    #[test]
    fn test11() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% if cond %}
            |{% if cond %}
            |EEEEEE
            ", "|").as_str()
        );

        assert!( t.is_err() );
        assert_eq!( t.unwrap_err().to_string(), "Parse error at end of text. Missing `{% end %}` /From Line:3 `{% if`" );
    }

    #[test]
    fn test12() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{% if cond
            |EEEEEE
            ", "|").as_str()
        );

        assert!( t.is_err() );
        assert_eq!( t.unwrap_err().to_string(), "Parse error at end of text. Missing `%}` /From Line:2 Col:0 `{%`" );
    }

    #[test]
    fn test13() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{#
            |{% if cond
            |EEEEEE
            ", "|").as_str()
        );

        assert!( t.is_err() );
        assert_eq!( t.unwrap_err().to_string(), "Parse error at end of text. Missing `#}` /From Line:2 `{#`" );
    }

    #[test]
    fn test14() {
        let mut c = Context::new();
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{#
            |BB {{ var.var1 }} BB {{ var.var2 }} BB
            |#}
            |EEEEEE
            ", "|").as_str()
        ).unwrap();

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );
    }

    #[test]
    fn test15() {
        let mut c = Context::new();
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{#
            |BB {{ var.var1 }} BB {{ var.var2 }} BB
            |{#
            |CCCCC
            |#}
            |#}
            |EEEEEE
            ", "|").as_str()
        ).unwrap();

        assert_eq!(
            trim_margin(
            "
            |AAAAAA
            |
            |EEEEEE
            ", "|").as_str(), t.render( &mut c ) );
    }

    #[test]
    fn test16() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{#
            |BB {{ var.var1 }} BB {{ var.var2 }} BB
            |{#
            |CCCCC
            |#}
            |EEEEEE
            ", "|").as_str()
        );

        assert!( t.is_err() );
        assert_eq!( t.unwrap_err().to_string(), "Parse error at end of text. Missing `#}` /From Line:2 `{#`" );
    }

    #[test]
    fn test17() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{#
            |BB {{ var.var1 }} BB {{ var.var2 }} BB
            |{#
            |CCCCC
            |#}
            |#}
            |
            |
            |#}
            |EEEEEE
            ", "|").as_str()
        );

        assert!( t.is_err() );
        assert_eq!( t.unwrap_err().to_string(), "Parse error at Line:10 Col:0. `#}`" );
    }

    #[test]
    fn test18() {
        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{{ var1 }}
            |{% if var2 %}
            |{{ var3 }}
            |{% else %}
            |{{ var4 }}
            |{{ var5 }}
            |{% end %}
            |{{ var6 }}
            |EEEEEE
            ", "|").as_str()
        ).unwrap();

        assert_eq!( t.get_varnames(), [ "var1", "var2", "var3", "var4", "var5", "var6" ].into_iter().map( String::from ).collect::<HashSet<String> >() );

        let t = parse_str(
            trim_margin(
            "
            |AAAAAA
            |{#
            |BB {{ var.var1 }} BB {{ var.var2 }} BB
            |{#
            |CCCCC
            |#}
            |{{ var2 }} {{ var3 }}
            |#}
            |{{ var4 }}
            |EEEEEE
            ", "|").as_str()
        ).unwrap();

        assert_eq!( t.get_varnames(), [ "var4" ].into_iter().map( String::from ).collect::<HashSet<String> >() );

    }

}
