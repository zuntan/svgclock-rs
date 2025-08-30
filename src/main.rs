extern crate pretty_env_logger;
#[macro_use]
extern crate log;
use log::{Level, log_enabled};
use rsvg::SvgHandle;
use strum::{IntoEnumIterator, VariantArray, VariantNames};

/* use std::sync::{Arc, Mutex}; */

use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use std::{io::Cursor, str};
use std::convert::AsRef;
use std::path::Path;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::sync::LazyLock;

use glam::{DAffine2, DMat2, DVec2, IVec2};
use quick_xml::events::BytesStart;
use regex::Regex;

use gtk::{ prelude::* };
use gtk::{ Application, ApplicationWindow, DrawingArea };
use gtk::{ Menu, MenuItem, CheckMenuItem, RadioMenuItem, SeparatorMenuItem };
use gtk::{ Dialog, DialogFlags, ResponseType, AboutDialog };

use gtk::cairo::{ Context, Rectangle, ImageSurface, Format, Region };
use gtk::cairo::{ FontSlant, FontWeight  };

use gtk::gdk::prelude::GdkSurfaceExt;
use gtk::gdk_pixbuf::Pixbuf;

use std::f64::consts::PI;

use chrono::{ DateTime, Local };
use chrono::{ Timelike, Utc };

use linked_hash_map::LinkedHashMap;

fn parse_float_list(val: &str) -> Vec<f64> {
    
    static RE_FLOAT: LazyLock<Regex> = LazyLock::new(
        || 
        {
            Regex::new(r"[-+]?([0-9]*\.[0-9]+|[0-9]+\.?[0-9]*)([eE][-+]?[0-9]+)?").unwrap()
        }
    );

    let arg: Vec<f64> = RE_FLOAT
        .captures_iter(val)
        .map(|x| f64::from_str(&x[0]))
        .filter(|x| if let Ok(_) = x { true } else { false })
        .map(|x| x.unwrap_or_default())
        .collect();

    arg
}

fn parse_svg_transform_value(transform: &str) -> Option<DAffine2> {
    
    static RE_TRANSLATE: LazyLock< Regex > = LazyLock::new(
        || 
        {
            Regex::new(r"(?i)(translate|scale|rotate|skewX|skewY|matrix)\s*\(([^\)]+)\)").unwrap()
        }
    );

    if String::from(transform).trim() == "" {
        None
    } else {
        let mut ret = DAffine2::IDENTITY;

        for caps in RE_TRANSLATE.captures_iter(transform) {
            let op = caps[1].to_lowercase();

            let arg: Vec<f64> = parse_float_list(&caps[2]);

            let m = match op.as_str() {
                "translate" if arg.len() == 2 => Some(DAffine2::from_translation(DVec2 {
                    x: arg[0],
                    y: arg[1],
                })),
                "scale" if arg.len() == 2 => Some(DAffine2::from_scale(DVec2 {
                    x: arg[0],
                    y: arg[1],
                })),
                "rotate" if arg.len() == 1 => Some(DAffine2::from_angle(arg[0])),
                "rotate" if arg.len() == 3 => Some(DAffine2::from_angle_translation(
                    arg[0],
                    DVec2 {
                        x: arg[1],
                        y: arg[2],
                    },
                )),
                "skewx" if arg.len() == 2 => None,
                "skewy" if arg.len() == 2 => None,
                "matrix" if arg.len() == 3 => Some(DAffine2::from_mat2_translation(
                    DMat2 {
                        x_axis: DVec2 {
                            x: arg[0],
                            y: arg[1],
                        },
                        y_axis: DVec2 {
                            x: arg[2],
                            y: arg[3],
                        },
                    },
                    DVec2 {
                        x: arg[4],
                        y: arg[5],
                    },
                )),
                _ => None,
            };

            if let Some(m) = m {
                ret *= m;
            }
        }

        Some(ret)
    }
}

type FilterOutput = Vec<u8>;
type FilterInputReader<'a> = quick_xml::Reader<&'a [u8]>;

fn parse_xml_sz_and_vbox(
    r_src: &mut FilterInputReader,
) -> Result<(IVec2, DVec2, DVec2), quick_xml::errors::Error> {
    let target_tag = "svg";

    let target_attr_key_width = "width";
    let target_attr_key_height = "height";
    let target_attr_key_viewbox = "viewBox";

    let mut sz = IVec2::ZERO;
    let mut viewbox_xy = DVec2::ZERO;
    let mut viewbox_sz = DVec2::ZERO;

    loop {
        let event = r_src.read_event();

        match event {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(quick_xml::events::Event::Start(ref tag)) => {
                if std::str::from_utf8(tag.name().as_ref()) == Ok(target_tag) {
                    if let Ok(attr) = tag.try_get_attribute(target_attr_key_width)
                        && let Some(attr) = attr
                    {
                        if let Ok(num) =
                            f64::from_str(std::str::from_utf8(attr.value.as_ref()).unwrap())
                        {
                            sz.x = num as i32;
                        }
                    }

                    if let Ok(attr) = tag.try_get_attribute(target_attr_key_height)
                        && let Some(attr) = attr
                    {
                        if let Ok(num) =
                            f64::from_str(std::str::from_utf8(attr.value.as_ref()).unwrap())
                        {
                            sz.y = num as i32;
                        }
                    }

                    if let Ok(attr) = tag.try_get_attribute(target_attr_key_viewbox)
                        && let Some(attr) = attr
                    {
                        let arg: Vec<f64> =
                            parse_float_list(std::str::from_utf8(attr.value.as_ref()).unwrap());

                        if arg.len() == 4 {
                            viewbox_xy = DVec2::new(arg[0], arg[1]);
                            viewbox_sz = DVec2::new(arg[2], arg[3]);

                            if sz.x == 0 {
                                sz.x = viewbox_sz.x as i32;
                            }

                            if sz.y == 0 {
                                sz.y = viewbox_sz.y as i32;
                            }
                        }
                    }

                    break;
                }
            }
            _ => {}
        }
    }

    Ok((sz, viewbox_xy, viewbox_sz))
}

fn parse_xml_center(r_src: &mut FilterInputReader) -> Result<DVec2, quick_xml::errors::Error> {
    let target_attr_key_transform = "transform";

    let target_tag = "g";
    let target_attr_key_groupmode = "inkscape:groupmode";
    let target_attr_val_groupmode = "layer";
    let target_attr_key_label = "inkscape:label";
    let target_attr_val_label = "center_circle";

    let target_tag_ellipse = "ellipse";
    let target_attr_key_cx = "cx";
    let target_attr_key_cy = "cy";

    let mut ret = DVec2::ZERO;

    let mut translate_affines: Vec<DAffine2> = Vec::new();

    let mut target_layer = false;

    let get_transform_affine = |tag: &BytesStart<'_>| {
        if let Ok(attr) = tag.try_get_attribute(target_attr_key_transform)
            && let Some(attr) = attr
        {
            if let Ok(attr_transform) = std::str::from_utf8(attr.value.as_ref()) {
                debug!("attr_translate:{:?}", attr_transform);

                if let Some(x) = parse_svg_transform_value(attr_transform) {
                    return x;
                }
            }
        }

        DAffine2::IDENTITY
    };

    loop {
        let event = r_src.read_event();

        match event {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(evt) => match evt {
                quick_xml::events::Event::Start(ref tag) => {
                    translate_affines.push(get_transform_affine(tag));

                    // check <g>

                    if translate_affines.len() == 2
                        && std::str::from_utf8(tag.name().as_ref()) == Ok(target_tag)
                    {
                        target_layer = false;

                        if let Ok(attr) = tag.try_get_attribute(target_attr_key_groupmode)
                            && let Some(attr) = attr
                        {
                            if std::str::from_utf8(attr.value.as_ref())
                                == Ok(target_attr_val_groupmode)
                            {
                                if let Ok(attr) = tag.try_get_attribute(target_attr_key_label)
                                    && let Some(attr) = attr
                                {
                                    if std::str::from_utf8(attr.value.as_ref())
                                        == Ok(target_attr_val_label)
                                    {
                                        target_layer = true;
                                    }
                                }
                            }
                        }
                    }
                }
                quick_xml::events::Event::Empty(ref tag) => {
                    debug!("target_layer: {:?} ", target_layer);
                    debug!("depth: {:?} ", translate_affines.len());
                    debug!("tag: {:?} ", tag);

                    if target_layer
                        && translate_affines.len() == 2
                        && std::str::from_utf8(tag.name().as_ref()) == Ok(target_tag_ellipse)
                    {
                        let mut tran_affine = DAffine2::IDENTITY;

                        debug!("tran_affine A: {:?}", tran_affine);

                        for x in &translate_affines {
                            tran_affine *= x;
                            debug!("tran_affine B: {:?}", tran_affine);
                        }

                        tran_affine *= get_transform_affine(tag);
                        debug!("tran_affine C: {:?}", tran_affine);

                        let mut vec2 = DVec2 { x: 0.0, y: 0.0 };

                        if let Ok(attr) = tag.try_get_attribute(target_attr_key_cx)
                            && let Some(attr) = attr
                        {
                            if let Ok(num) =
                                f64::from_str(std::str::from_utf8(attr.value.as_ref()).unwrap())
                            {
                                vec2.x = num;
                            }
                        }

                        if let Ok(attr) = tag.try_get_attribute(target_attr_key_cy)
                            && let Some(attr) = attr
                        {
                            if let Ok(num) =
                                f64::from_str(std::str::from_utf8(attr.value.as_ref()).unwrap())
                            {
                                vec2.y = num;
                            }
                        }

                        debug!("vec2: {:?}", vec2);

                        ret = tran_affine.transform_point2(vec2);

                        debug!("ret: {:?}", ret);
                    }
                }
                quick_xml::events::Event::End(ref tag) => {
                    translate_affines.pop();
                }
                _ => {}
            },

            Err(e) => panic!("Error at position {}: {:?}", r_src.error_position(), e),
        }
    }

    Ok(ret)
}

enum FilterTarget {
    Base,
    LongHandle,
    ShortHandle,
    SecondHandle,
    CenterCircle,
}

fn filter_xml(
    r_src: &mut FilterInputReader,
    t: FilterTarget,
) -> Result<FilterOutput, quick_xml::errors::Error> {
    let target_tag = "g";
    let target_attr_key_groupmode = "inkscape:groupmode";
    let target_attr_val_groupmode = "layer";
    let target_attr_key_label = "inkscape:label";

    let target = match t {
        FilterTarget::Base => "base",
        FilterTarget::LongHandle => "long_handle",
        FilterTarget::ShortHandle => "short_handle",
        FilterTarget::SecondHandle => "second_handle",
        FilterTarget::CenterCircle => "center_circle",
    };

    let mut writer = quick_xml::Writer::new(Cursor::new(Vec::<u8>::new()));

    let mut depth = 0;
    let mut depth_dis_output = -1;

    loop {
        let event = r_src.read_event();

        match event {
            Ok(quick_xml::events::Event::Eof) => break,

            Ok(evt) => match evt {
                quick_xml::events::Event::Start(ref tag) => {
                    depth += 1;

                    if depth == 2 && std::str::from_utf8(tag.name().as_ref()) == Ok(target_tag) {
                        let mut output = false;

                        if let Ok(attr) = tag.try_get_attribute(target_attr_key_groupmode)
                            && let Some(attr) = attr
                        {
                            if std::str::from_utf8(attr.value.as_ref())
                                == Ok(target_attr_val_groupmode)
                            {
                                if let Ok(attr) = tag.try_get_attribute(target_attr_key_label)
                                    && let Some(attr) = attr
                                {
                                    if std::str::from_utf8(attr.value.as_ref()) == Ok(target) {
                                        output = true;
                                    }
                                }
                            }
                        }

                        depth_dis_output = if output { -1 } else { depth };
                    }

                    if depth_dis_output == -1 {
                        assert!(writer.write_event(evt).is_ok())
                    }
                }
                quick_xml::events::Event::End(ref tag) => {
                    if depth_dis_output == -1 {
                        assert!(writer.write_event(evt).is_ok())
                    } else if depth == depth_dis_output {
                        depth_dis_output = -1;
                    }

                    depth -= 1;
                }
                _ => {
                    if depth_dis_output == -1 {
                        assert!(writer.write_event(evt.borrow()).is_ok())
                    }
                }
            },

            Err(e) => panic!("Error at position {}: {:?}", r_src.error_position(), e),
        }
    }

    Ok(writer.into_inner().into_inner())
}

struct ImageInfo {
    sz: IVec2,
    viewbox_xy: DVec2,
    viewbox_sz: DVec2,

    bytes_base: Option<Vec<u8>>,
    bytes_long_handle: Option<Vec<u8>>,
    bytes_short_handle: Option<Vec<u8>>,
    bytes_second_handle: Option<Vec<u8>>,
    bytes_center_circle: Option<Vec<u8>>,

    svgh_base: Option<SvgHandle>,
    svgh_long_handle: Option<SvgHandle>,
    svgh_short_handle: Option<SvgHandle>,
    svgh_second_handle: Option<SvgHandle>,
    svgh_center_circle: Option<SvgHandle>,

    center: DVec2,
}

impl ImageInfo {
    const fn new() -> Self {
        Self {
            sz: IVec2::ZERO,
            viewbox_xy: DVec2::ZERO,
            viewbox_sz: DVec2::ZERO,

            bytes_base: None,
            bytes_long_handle: None,
            bytes_short_handle: None,
            bytes_second_handle: None,
            bytes_center_circle: None,

            svgh_base: None,
            svgh_long_handle: None,
            svgh_short_handle: None,
            svgh_second_handle: None,
            svgh_center_circle: None,

            center: DVec2::ZERO,
        }
    }
}

fn load_theme( theme: AppInfoTheme, theme_custom: Option< String > ) -> Option< ImageInfo >
{
    let src_buf: Option< Vec<u8> > = 
        match theme
        {
            AppInfoTheme::Theme1 => 
            {
                let mut src_buf = Vec::<u8>::new();
                let mut src = File::open("clock_theme_1.svg").unwrap();
                src.read_to_end( &mut src_buf ).unwrap();

                Some( src_buf )
            }

            AppInfoTheme::Theme2 => 
            {
                let mut src_buf = Vec::<u8>::new();
                let mut src = File::open("clock_theme_2.svg").unwrap();
                src.read_to_end( &mut src_buf ).unwrap();

                Some( src_buf )
            }

            AppInfoTheme::Theme3 => 
            {
                let mut src_buf = Vec::<u8>::new();
                let mut src = File::open("clock_theme_3.svg").unwrap();
                src.read_to_end( &mut src_buf ).unwrap();

                Some( src_buf )
            }

            AppInfoTheme::Custom =>
            {
                if let Some( theme_custom ) = theme_custom
                {
                    let mut src_buf = Vec::<u8>::new();
                    let src = File::open(theme_custom );

                    if let Ok( mut src ) = src
                    {
                        if let Ok(_) = src.read_to_end( &mut src_buf )
                        {
                            Some( src_buf )
                        }
                        else { None }
                    }
                    else { None }
                }
                else { None }
            }
        }
        ;
    
    if let Some( src_buf ) = src_buf
    {
        Some( load_xml( &src_buf) )
    }
    else 
    {
        None
    }
}

fn load_xml( src_buf: & Vec<u8> ) -> ImageInfo 
{
    let src_base = filter_xml(
        &mut FilterInputReader::from_reader(&src_buf),
        FilterTarget::Base,
    );
    let src_long_handle = filter_xml(
        &mut FilterInputReader::from_reader(&src_buf),
        FilterTarget::LongHandle,
    );
    let src_short_handle = filter_xml(
        &mut FilterInputReader::from_reader(&src_buf),
        FilterTarget::ShortHandle,
    );
    let src_second_handle = filter_xml(
        &mut FilterInputReader::from_reader(&src_buf),
        FilterTarget::SecondHandle,
    );
    let src_center_circle = filter_xml(
        &mut FilterInputReader::from_reader(&src_buf),
        FilterTarget::CenterCircle,
    );

    let fn_make_svg_handle = | src_xml : &Vec<u8> | 
    {
        let svg_stream = gtk::gio::MemoryInputStream::from_bytes(&gtk::glib::Bytes::from( src_xml ));

        Some(
            rsvg::Loader::new()
            .read_stream(
                &svg_stream,
                None::<&gtk::gio::File>,
                None::<&gtk::gio::Cancellable>,
            )
            .unwrap()
        )
    };

    let mut ret = ImageInfo::new();

    if let Ok(src_xml) = src_base {
        if let Ok(result) = parse_xml_sz_and_vbox(&mut FilterInputReader::from_reader(&src_xml)) {
            ret.sz = result.0;
            ret.viewbox_xy = result.1;
            ret.viewbox_sz = result.2;
        }

        ret.svgh_base = fn_make_svg_handle( &src_xml );
        ret.bytes_base = Some(src_xml);
    }

    if let Ok(src_xml) = src_long_handle {
        ret.svgh_long_handle = fn_make_svg_handle( &src_xml );
        ret.bytes_long_handle = Some(src_xml);
    }

    if let Ok(src_xml) = src_short_handle {
        ret.svgh_short_handle = fn_make_svg_handle( &src_xml );
        ret.bytes_short_handle = Some(src_xml);
    }

    if let Ok(src_xml) = src_second_handle {
        ret.svgh_second_handle = fn_make_svg_handle( &src_xml );
        ret.bytes_second_handle = Some(src_xml);
    }

    if let Ok(src_xml) = src_center_circle {

        if let Ok(center) = parse_xml_center(&mut FilterInputReader::from_reader(&src_xml)) {
            ret.center = center;
        }

        debug!("ret.center: {:?}", ret.center);

        ret.svgh_center_circle = fn_make_svg_handle( &src_xml );
        ret.bytes_center_circle = Some(src_xml);
    }

    ret
}

fn load_logo() -> Option< Pixbuf >
{
    // load logo
    let mut src_buf = Vec::<u8>::new();
    
    let mut src = File::open("logo.svg").unwrap();
    src.read_to_end(&mut src_buf).unwrap();

    if let Ok(result) = parse_xml_sz_and_vbox(&mut FilterInputReader::from_reader(&src_buf)) {

        let sz = result.0;
        let surface = ImageSurface::create(Format::ARgb32, sz.x, sz.y ).unwrap();
        
        {
            let svg_stream = gtk::gio::MemoryInputStream::from_bytes(&gtk::glib::Bytes::from( &src_buf ));

            let svg_handle = 
                rsvg::Loader::new()
                .read_stream(
                    &svg_stream,
                    None::<&gtk::gio::File>,
                    None::<&gtk::gio::Cancellable>,
                )
                .unwrap()
                ;

            let cctx = Context::new( &surface ).unwrap();
            let viewport = Rectangle::new(0.0, 0.0, sz.x as f64, sz.y as f64);

            let svg_renderer = rsvg::CairoRenderer::new( &svg_handle );
            svg_renderer.render_document( &cctx, &viewport ).unwrap();
        }

        return gdk::pixbuf_get_from_surface( &surface, 0, 0, surface.width(), surface.height() );
    }

    None
}

fn make_region( image_info: &ImageInfo, sz: DVec2 ) -> Option< Region >
{
    if let Some(src_xml) = image_info.bytes_base.as_ref()
        && image_info.sz.x > 0
        && image_info.sz.y > 0
        && sz.x > 0.0
        && sz.y > 0.0
    {
        let svg_stream = gtk::gio::MemoryInputStream::from_bytes(&gtk::glib::Bytes::from(src_xml));
        let svg_handle = rsvg::Loader::new()
            .read_stream(
                &svg_stream,
                None::<&gtk::gio::File>,
                None::<&gtk::gio::Cancellable>,
            )
            .unwrap();
        let svg_renderer = rsvg::CairoRenderer::new(&svg_handle);

        let surface_mask = ImageSurface::create(Format::A8, sz.x as i32, sz.y as i32 ).unwrap();

        let cctx = Context::new(&surface_mask).unwrap();

        let viewport = Rectangle::new(0.0, 0.0, sz.x, sz.y );

        debug!("viewport:{:?}", viewport);

        svg_renderer.render_document(&cctx, &viewport).unwrap();

        let mut mask_file = File::create("mask.png").unwrap();

        surface_mask.write_to_png(&mut mask_file).unwrap();

        surface_mask.create_region()
    }
    else 
    {
        None
    }
}

fn update_region<'a>( window: &'a ApplicationWindow, image_info: &'a ImageInfo, app_info: &'a mut AppInfo ) 
{
    if app_info.zoom_update
    {
        if  image_info.sz.x > 0
        &&  image_info.sz.y > 0
        &&  app_info.zoom > 0
        {
            let zoom_factor = app_info.zoom as f64 / 100.0;

            let sz = DVec2::new( 
                image_info.sz.x as f64 * zoom_factor
            ,   image_info.sz.y as f64 * zoom_factor
            );

            window.resize( sz.x as i32, sz.y as i32 );
            window.shape_combine_region( make_region( &image_info, sz ).as_ref() );

            app_info.zoom_update = false;
        }
    }
}

#[derive(Debug, PartialEq, strum::EnumString, strum::Display, strum::EnumIter, Copy, Clone )]
enum AppInfoTheme
{
    Theme1
,   Theme2
,   Theme3    
,   Custom
}

struct AppInfo
{
    always_on_top: bool
,   lock_pos: bool
,   show_seconds: bool
,   enable_sub_seconds: bool
,   show_date: bool
,   time_zone: String
,   theme: AppInfoTheme
,   theme_custome: Option< String >
,   zoom: u32
,   zoom_update: bool
}

impl AppInfo 
{
    const fn new() -> Self {
        Self
        {
            always_on_top: true
        ,   lock_pos: false
        ,   show_seconds: true
        ,   enable_sub_seconds: false
        ,   show_date: false
        ,   time_zone: String::new()
        ,   theme: AppInfoTheme::Theme1
        ,   theme_custome: None
        ,   zoom: 100
        ,   zoom_update: true
        }
    }
}

fn draw<'a>( cctx : &'a Context, image_info : &'a ImageInfo, app_info: &'a AppInfo )
{
    let zoom_factor = app_info.zoom as f64 / 100.0;

    let sz = DVec2::new( 
        image_info.sz.x as f64 * zoom_factor
    ,   image_info.sz.y as f64 * zoom_factor
    );

    let viewport = Rectangle::new(0.0, 0.0, sz.x, sz.y );
    let center = DVec2
    {
        x: sz.x * ( image_info.center.x / image_info.viewbox_sz.x )
    ,   y: sz.y * ( image_info.center.y / image_info.viewbox_sz.y )
    };

    if let Some( x ) = image_info.svgh_base.as_ref()
    {
        let svg_renderer = rsvg::CairoRenderer::new(x);
        svg_renderer.render_document(cctx, &viewport).unwrap();
    }

    let func_rotate = | svg_handle : &SvgHandle, angle : f64 |
    {
        let _ = cctx.save();

        cctx.translate( center.x * 1.0, center.y * 1.0 );
        cctx.rotate( angle * ( PI / 180.0 ) );
        cctx.translate( center.x * -1.0, center.y * -1.0 );

        let svg_renderer = rsvg::CairoRenderer::new( svg_handle );
        svg_renderer.render_document(cctx, &viewport).unwrap();

        let _ = cctx.restore();
    };

    let local_datetime: DateTime<Local> = Local::now();

    let secs
        = ( local_datetime.hour() * 60 * 60
        + local_datetime.minute() * 60
        + local_datetime.second()
        ) as f64
        ;

    let angle_hour = secs / ( 12.0 * 60.0 * 60.0 ) * 360.0;
    let angle_min = secs / ( 60.0 * 60.0 ) * 360.0;
    let angle_sec = local_datetime.second() as f64 / 60.0 * 360.0;

    if let Some( x ) = image_info.svgh_long_handle.as_ref()
    {
        func_rotate( x, angle_min );
    }    

    if let Some( x ) = image_info.svgh_short_handle.as_ref()
    {
        func_rotate( x, angle_hour );
    }    

    if app_info.show_seconds
    {
        if let Some( x ) = image_info.svgh_second_handle.as_ref()
        {
            func_rotate( x, angle_sec );
        }   
    }

    if let Some( x ) = image_info.svgh_center_circle.as_ref()
    {
        func_rotate( x, angle_sec );
    }   

}

fn make_theme_menu(
    win: &ApplicationWindow, 
    da: &DrawingArea, 
    image_info: &Rc< RefCell< ImageInfo > >,
    app_info: &Rc< RefCell< AppInfo > >    
) -> Menu
{
    let menu = Menu::new();

    for ait in AppInfoTheme::iter()
    {
        let menu_item = CheckMenuItem::with_label( ait.to_string().as_str() );          

        menu_item.set_active( ait == app_info.borrow().theme );

        let _da = da.clone();
        let _app_info = app_info.clone();    
        let _image_info = image_info.clone();

        menu_item.connect_activate(
            move |_| 
            {
                let mut _app_info = _app_info.borrow_mut();
                _app_info.theme = ait;
                _app_info.zoom = 100;
                _app_info.zoom_update = true;
                _image_info.replace( load_theme( _app_info.theme, _app_info.theme_custome.clone() ).unwrap() );
            }  
        );

        if ait == AppInfoTheme::Custom
        {
            menu_item.set_sensitive( app_info.borrow().theme_custome.is_some() );
        }

        menu.append( &menu_item );        
    }

    menu      
}

fn make_zoom_menu(
    da: &DrawingArea, 
    app_info: &Rc< RefCell< AppInfo > >    
) -> Menu
{
    static ZOOMS: LazyLock< Vec<u32> > = LazyLock::new( 
        || 
        {
            (30..=230).step_by(10).collect()
        }
    );

    let _zoom = app_info.borrow().zoom;

    if ZOOMS.iter().find( | &&x| { x == _zoom } ).is_none()
    {
        app_info.borrow_mut().zoom = 100;
    }

    let menu = Menu::new();

    for &x in ZOOMS.iter()
    {
        let label = format!( "{}%", x );

        let menu_item = CheckMenuItem::with_label( label.as_str() );           

        menu_item.set_active( app_info.borrow().zoom == x );

        let _da = da.clone();
        let _app_info = app_info.clone();    
        menu_item.connect_activate(
            move |_| {
                let mut _app_info = _app_info.borrow_mut();
                _app_info.zoom = x;
                _app_info.zoom_update = true;
            }  
        );

        menu.append( &menu_item );
    }

    menu    
}

fn make_timezone_menu(
    da: &DrawingArea, 
    app_info: &Rc< RefCell< AppInfo > >    
) -> Menu
{
    // first parse

    let mut dic = LinkedHashMap::< &str, Vec< &str > >::new();

    for x in chrono_tz::TZ_VARIANTS
    {
        let n = x.name();
        let p:Vec<_> = n.split( "/" ).collect();
        
        if p.len() <= 2
        {
            let area = p[0];

            if ! dic.contains_key( area )
            {
                dic.insert( area, Vec::<_>::new() );
            }

            let mut x = Vec::<_>::new();
            x.push( "" );

            if p.len() == 2
            {
                if let Some( vec ) = dic.get_mut( area )
                {
                    let city = p[1];
                    if ! ( city.starts_with( "GMT" ) || city.starts_with( "UTC" ) || city.starts_with( "UCT" ) )
                    {
                        vec.push( city );
                    }
                }
            }
        }
    }

    //debug!("{:?}", dic );

    let menu = Menu::new();

    let tz = app_info.borrow().time_zone.clone();
    let tz = if tz == "" { String::from( "<< (Local Time) >>" ) } else { format!("<< {} >>", tz) };

    let menu_item_now = MenuItem::with_label( tz.as_str() );
    menu_item_now.set_sensitive( false );

    menu.append( &menu_item_now );
    menu.append( &SeparatorMenuItem::new() );

    let menu_item_local_time = CheckMenuItem::with_label( "Local Time" );

    menu_item_local_time.set_active( app_info.borrow().time_zone == "" );
    
    let _da = da.clone();
    let _app_info = app_info.clone();    
    menu_item_local_time.connect_activate(
        move |_| {
            let mut _app_info = _app_info.borrow_mut();
            _app_info.time_zone = String::from( "" );
            _da.queue_draw();
        }  
    );

    menu.append( &menu_item_local_time );

    let menu_item_utc = CheckMenuItem::with_label( "UTC" );

    menu_item_utc.set_active( app_info.borrow().time_zone == "UTC" );
    
    let _da = da.clone();
    let _app_info = app_info.clone();    
    menu_item_utc.connect_activate(
        move |_| {
            let mut _app_info = _app_info.borrow_mut();
            _app_info.time_zone = String::from( "UTC" );
            _da.queue_draw();
        }  
    );

    menu.append( &menu_item_utc );

    let menu_item_gmt = MenuItem::with_label( "Greenwich Mean Time" );

    let menu_gmt = Menu::new();

    for gmt_entry in [
        "GMT-12", "GMT-11", "GMT-10"
    ,   "GMT-9", "GMT-8", "GMT-7", "GMT-6", "GMT-5", "GMT-4", "GMT-3", "GMT-2", "GMT-1"
    ,   "GMT"
    ,   "GMT+1", "GMT+2", "GMT+3", "GMT+4", "GMT+5", "GMT+6", "GMT+7", "GMT+8", "GMT+9" 
    ,   "GMT+11", "GMT+12", "GMT+13", "GMT+14"
    ]
    {
        let menu_item_gmt_entry = CheckMenuItem::with_label( gmt_entry );

        menu_item_gmt_entry.set_active( app_info.borrow().time_zone == gmt_entry );

        let _da = da.clone();
        let _app_info = app_info.clone();    
        menu_item_gmt_entry.connect_activate(
            move |_| {
                let mut _app_info = _app_info.borrow_mut();
                _app_info.time_zone = String::from( gmt_entry );
                _da.queue_draw();
            }  
        );

        menu_gmt.append( &menu_item_gmt_entry );
    }

    menu_item_gmt.set_submenu( Some( &menu_gmt ) );

    menu.append( &menu_item_gmt );
    menu.append( &SeparatorMenuItem::new() );

    for ( area, cities ) in dic
    {
        if !cities.is_empty()
        {
            let menu_item_area = MenuItem::with_label( area );
            let menu_area  = Menu::new();

            for city in cities
            {
                let tz = format!("{}/{}", area, city );

                let menu_item_city = CheckMenuItem::with_label( city );           

                menu_item_city.set_active( app_info.borrow().time_zone == tz );

                let _da = da.clone();
                let _app_info = app_info.clone();    
                menu_item_city.connect_activate(
                    move |_| {
                        let mut _app_info = _app_info.borrow_mut();
                        _app_info.time_zone = tz.clone();
                        _da.queue_draw();
                    }  
                );                

                menu_area.append( &menu_item_city );
            }

            menu_item_area.set_submenu( Some( &menu_area ) );

            menu.append( &menu_item_area );
        }
    }

    menu
}

fn make_popup_menu( 
    app: &Application, 
    win: &ApplicationWindow,
    da: &DrawingArea, 
    app_info: &Rc< RefCell< AppInfo > >,
    image_info: &Rc< RefCell< ImageInfo > >,
    logo: Option<Pixbuf>
) -> Menu
{
    let menu = Menu::new();

    let menu_item_pref = MenuItem::with_label( "Preferances" );

    let menu_item_pref_alway_on_top = CheckMenuItem::with_label( "Alway on Top" );

    menu_item_pref_alway_on_top.set_active( app_info.borrow().always_on_top );

    let _win = win.clone();
    let _da = da.clone();
    let _app_info = app_info.clone();

    menu_item_pref_alway_on_top.connect_activate( move |_| 
        {
            let mut _app_info = _app_info.borrow_mut();
            _app_info.always_on_top = !_app_info.always_on_top;
            _win.set_keep_above( _app_info.always_on_top );
            _da.queue_draw();
        }
    );

    let menu_item_pref_lock_pos = gtk::CheckMenuItem::with_label( "Lock Position" );

    menu_item_pref_lock_pos.set_active( app_info.borrow().lock_pos );
    
    let _da = da.clone();
    let _app_info = app_info.clone();

    menu_item_pref_lock_pos.connect_activate( move |_| 
        {
            let mut _app_info = _app_info.borrow_mut();
            _app_info.lock_pos = !_app_info.lock_pos;
            _da.queue_draw();
        }
    );

    let menu_item_pref_show_seconds = gtk::CheckMenuItem::with_label( "Show Seconds" );

    menu_item_pref_show_seconds.set_active( app_info.borrow().show_seconds );
    
    let _da = da.clone();
    let _app_info = app_info.clone();

    menu_item_pref_show_seconds.connect_activate( move |_| 
        {
            let mut _app_info = _app_info.borrow_mut();
            _app_info.show_seconds = !_app_info.show_seconds;
            _da.queue_draw();
        }
    );
    
    let menu_item_pref_enable_sub_seconds = gtk::CheckMenuItem::with_label( "Enable Sub Seconds" );

    menu_item_pref_enable_sub_seconds.set_active( app_info.borrow().enable_sub_seconds );
    
    let _da = da.clone();
    let _app_info = app_info.clone();

    menu_item_pref_enable_sub_seconds.connect_activate( move |_| 
        {
            let mut _app_info = _app_info.borrow_mut();
            _app_info.enable_sub_seconds = !_app_info.enable_sub_seconds;
            _da.queue_draw();
        }
    );

    let menu_item_pref_show_date = gtk::CheckMenuItem::with_label( "Show Date" );

    menu_item_pref_show_date.set_active( app_info.borrow().show_date );
    
    let _da = da.clone();
    let _app_info = app_info.clone();

    menu_item_pref_show_date.connect_activate( move |_| 
        {
            let mut _app_info = _app_info.borrow_mut();
            _app_info.show_date = !_app_info.show_date;
            _da.queue_draw();
        }
    );

    let menu_item_pref_time_zone = MenuItem::with_label( "Time Zone" );
    let menu_item_pref_theme = MenuItem::with_label( "Theme" );
    let menu_item_pref_zoom = MenuItem::with_label( "Zoom" );

    let menu_pref = Menu::new();

    menu_pref.append( &menu_item_pref_alway_on_top );
    menu_pref.append( &menu_item_pref_lock_pos );
    menu_pref.append( &SeparatorMenuItem::new() );
    menu_pref.append( &menu_item_pref_show_seconds );
    menu_pref.append( &menu_item_pref_enable_sub_seconds );
    menu_pref.append( &menu_item_pref_show_date );
    menu_pref.append( &SeparatorMenuItem::new() );
    menu_pref.append( &menu_item_pref_time_zone );
    menu_pref.append( &menu_item_pref_theme );
    menu_pref.append( &menu_item_pref_zoom );

    menu_item_pref.set_submenu( Some( &menu_pref ) );

    let menu_pref_time_zone = make_timezone_menu( &da.clone(), &app_info.clone() );
    menu_item_pref_time_zone.set_submenu( Some( &menu_pref_time_zone ) );

    let menu_pref_theme = make_theme_menu( &win.clone(), &da.clone(), &image_info.clone(), &app_info.clone() );
    menu_item_pref_theme.set_submenu( Some( &menu_pref_theme ) );

    let menu_pref_zoom = make_zoom_menu( &da.clone(), &app_info.clone() );
    menu_item_pref_zoom.set_submenu( Some( &menu_pref_zoom ) );

    let menu_item_about = MenuItem::with_label( "About" );

    let _win = win.clone();
    let _image_info = image_info.clone();

    menu_item_about.connect_activate(
        move |_| 
        {
            let about_dialog = AboutDialog::builder()
                .title( "title:hello_gtk" )
                .program_name( "hello_gtk" )
                .comments( "hello_gtk is a analogue clock." )
                .copyright( "Copyright © 2025 zuntan <>" )
                .version( "version" )
                .website( "https://github.com/zuntan/" )
                .authors( [ "authors:zuntan", ] )
                .artists( [ "artists:zuntan", ] )
                .modal( true )
                .destroy_with_parent( true )
                .build()
                ;
            
            about_dialog.set_logo( logo.as_ref() );    
            about_dialog.set_parent( &_win );
            about_dialog.show_all();
        }
    );

    let menu_item_quit = MenuItem::with_label( "Quit");

    let _app = app.clone();

    menu_item_quit.connect_activate(
        move |_| {
            _app.quit();
        }
    );

    menu.append( &menu_item_pref );
    menu.append( &SeparatorMenuItem::new() );
    menu.append( &menu_item_about );
    menu.append( &menu_item_quit );    

    menu
}

fn main() {
    pretty_env_logger::init();

    let app = Application::builder()
        .application_id("net.zuntan.example")
        .build();

    app.connect_activate(move |app| {

        let app_info = Rc::new( RefCell::new( AppInfo::new() ) );

        let image_info = Rc::new( RefCell::new( load_theme( AppInfoTheme::Theme1, None ).unwrap() ) );

        let window = ApplicationWindow::builder()
            .application(app)
            .title("net.zuntan.example")
            .decorated(false)
            .tooltip_markup("example")
/* 
            .default_width(image_info.sz.x)
            .default_height(image_info.sz.y)
*/
            .build();

        window.set_keep_above( app_info.borrow().always_on_top );


        let da =  DrawingArea::new();

        let _window = window.clone();
        let _image_info = image_info.clone();
        let _app_info = app_info.clone();

        da.connect_draw( move | _, cr | 
            {
                update_region( &_window, &_image_info.borrow(), &mut _app_info.borrow_mut() );
                draw( cr, &_image_info.borrow(), &_app_info.borrow() );
                gtk::glib::Propagation::Proceed
            }
        );

        window.add(&da);

        let _app = app.clone();
        let _window = window.clone();
        let _da = da.clone();
        let _image_info = image_info.clone();
        let _app_info = app_info.clone();

        window.connect_button_press_event( move | window,  evt | 
            {
                log::debug!("pressed: {:?}", evt.button() );

                let logo = load_logo();

                match evt.button()
                {
                    1 => /* left button */
                    {
                        if !_app_info.borrow().lock_pos
                        {
                            let btn = evt.as_ref();
                            window.begin_move_drag( btn.button as i32, btn.x_root as i32, btn.y_root as i32, btn.time );
                        }
                    }
                ,   3 => /* right button */
                    {
                        let menu = make_popup_menu( &_app, &_window,&_da, &_app_info, &_image_info, logo );

                        menu.show_all();
                        menu.popup_at_pointer( Some( evt ) );

                        return gtk::glib::Propagation::Stop;
                    }
                ,   _ => {}
                }
                
                gtk::glib::Propagation::Proceed
            }
        );

        window.show_all();

        gtk::glib::source::timeout_add_local(std::time::Duration::from_millis(100), move || 
            {
                /* log::debug!("timeout" );         */
                da.queue_draw();
                gtk::glib::ControlFlow::Continue
            }
        );
    });

    app.run();
}
