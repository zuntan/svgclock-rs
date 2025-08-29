use glam::{ Affine2, Vec2, DAffine2, DVec2, DMat2 };
use regex::Regex;
use std::str::FromStr;

fn main()
{
    println!( "hello" );

    let a = Affine2::from_translation( Vec2 { x:10.0, y:10.0 } );
    let b = a.inverse();

    println!( "a : {}", a );
    println!( "b : {}", b );

    let v =  Vec2 { x:10.0, y:10.0 };

    println!( "v : {}", v );

    let vv =  a.transform_point2( v );

    println!( "vv : {}", vv );

    let vv =  b.transform_point2( v );

    println!( "vv : {}", vv );

    let vv =  a.transform_point2( b.transform_point2( v ) );

    println!( "vv : {}", vv );

    println!( "v : {}", v );

    let vv =  a.transform_point2( Vec2 { x: 10.0, y: 0.0 } );

    println!( "vv : {}", vv );
    println!( "a : {}", a );    


    println!( "a * b : {}", a * b );    

    let src = "scale(1,1) rotate(45 100 330) skewX(10) skewY(10) translate(20,0) matrix( a,b,c,d,e,f )";

    let re = Regex::new(r"(?i)(translate|scale|rotate|skewX|skewY|matrix)\s*\(([^\)]+)\)").unwrap();
    let reF = Regex::new(r"[-+]?([0-9]*\.[0-9]+|[0-9]+\.?[0-9]*)([eE][-+]?[0-9]+)?").unwrap();

    let mut mat = DAffine2::IDENTITY;

    for caps in re.captures_iter(src) 
    {
        let arg: Vec<f64>  = reF.captures_iter(&caps[2])
            .map( | x | f64::from_str( &x[0] ) )
            .filter( |x| if let Ok(_) = x { true } else { false } )
            .map( | x | { x.unwrap_or_default() } )
            .collect()
            ;

        let op = caps[1].to_lowercase();
    for caps in re.captures_iter(src) 
    {
        let arg: Vec<f64>  = reF.captures_iter(&caps[2])
            .map( | x | f64::from_str( &x[0] ) )
            .filter( |x| if let Ok(_) = x { true } else { false } )
            .map( | x | { x.unwrap_or_default() } )
            .collect()
            ;

        let op = caps[1].to_lowercase();

        let m = 
            match op.as_str()
            {
                "translate" if arg.len() == 2 => 
                {
                    Some( DAffine2::from_translation( DVec2 { x: arg[0], y: arg[1] } ) )
                }
            ,   "scale" if arg.len() == 2 => 
                {
                    Some( DAffine2::from_scale( DVec2 { x: arg[0], y: arg[1] } ) )
                }
            ,   "rotate" if arg.len() == 1 => 
                {
                    Some( DAffine2::from_angle( arg[0] ) )
                }  
            ,   "rotate" if arg.len() == 3 => 
                {
                    Some( DAffine2::from_angle_translation( arg[0], DVec2 { x: arg[1], y: arg[2] } ) )
                }  
            ,   "skewx" if arg.len() == 2 => { None }
            ,   "skewy" if arg.len() == 2 => { None }
            ,   "matrix" if arg.len() == 3 => 
                {
                    Some( DAffine2::from_mat2_translation(
                         DMat2
                        { 
                            x_axis: DVec2 { x: arg[0], y: arg[1] }
                        ,   y_axis: DVec2 { x: arg[2], y: arg[3] }
                        }
                    ,   DVec2 { x: arg[4], y: arg[5] }
                    ) )
                }
            ,   _ => { None }
            };

        if let Some( m ) = m
        {
            mat *= m;
        }

        println!("{}", &caps[0]);
        println!("{}", &caps[1]);
        println!("{}", &caps[2]);
        println!("{:?}", arg );
    }

    println!("{:?}", mat );
    println!("{:?}", mat == DAffine2::IDENTITY );

        let m = 
            match op.as_str()
            {
                "translate" if arg.len() == 2 => 
                {
                    Some( DAffine2::from_translation( DVec2 { x: arg[0], y: arg[1] } ) )
                }
            ,   "scale" if arg.len() == 2 => 
                {
                    Some( DAffine2::from_scale( DVec2 { x: arg[0], y: arg[1] } ) )
                }
            ,   "rotate" if arg.len() == 1 => 
                {
                    Some( DAffine2::from_angle( arg[0] ) )
                }  
            ,   "rotate" if arg.len() == 3 => 
                {
                    Some( DAffine2::from_angle_translation( arg[0], DVec2 { x: arg[1], y: arg[2] } ) )
                }  
            ,   "skewx" if arg.len() == 2 => { None }
            ,   "skewy" if arg.len() == 2 => { None }
            ,   "matrix" if arg.len() == 3 => 
                {
                    Some( DAffine2::from_mat2_translation(
                         DMat2
                        { 
                            x_axis: DVec2 { x: arg[0], y: arg[1] }
                        ,   y_axis: DVec2 { x: arg[2], y: arg[3] }
                        }
                    ,   DVec2 { x: arg[4], y: arg[5] }
                    ) )
                }
            ,   _ => { None }
            };

        if let Some( m ) = m
        {
            mat *= m;
        }

        println!("{}", &caps[0]);
        println!("{}", &caps[1]);
        println!("{}", &caps[2]);
        println!("{:?}", arg );
    }

    println!("{:?}", mat );
    println!("{:?}", mat == DAffine2::IDENTITY );
    

}