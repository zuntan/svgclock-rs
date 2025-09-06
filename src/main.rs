extern crate pretty_env_logger;
#[macro_use]
extern crate log;

/* use std::sync::{Arc, Mutex}; */

use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use std::{io::Cursor, str};
use std::convert::AsRef;
use std::error::Error;

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::LazyLock;
use std::f64::consts::PI;

// use log::{Level, log_enabled};

use strum::{ IntoEnumIterator };

use serde::{Deserialize, Serialize};

use glam::{DAffine2, DMat2, DVec2, IVec2};
use quick_xml::events::{BytesRef, BytesStart};
use regex::Regex;

use gtk::{ prelude::* };
use gtk::{ Application, ApplicationWindow, DrawingArea };
use gtk::{ Menu, MenuItem, CheckMenuItem,  SeparatorMenuItem };
use gtk::{ AboutDialog, /* Dialog, DialogFlags, ResponseType */ };

use gtk::cairo::{ Context, Rectangle, ImageSurface, Format, Region };
// use gtk::cairo::{ FontSlant, FontWeight  };

use gtk::gdk::prelude::GdkSurfaceExt;
use gtk::gdk_pixbuf::Pixbuf;

use rsvg::SvgHandle;

use chrono::{ DateTime, Local, NaiveDateTime, NaiveTime, TimeDelta, Timelike, Utc };

use linked_hash_map::LinkedHashMap;

fn to_char( br: &BytesRef ) -> Option< char >
{
    if let Ok( x ) = br.resolve_char_ref() && let Some( x ) = x
    {
        return Some( x );
    }
    else if let Ok( x ) = br.xml_content()
    {
        match x.to_lowercase().as_str()
        {
            "amp" => { return Some('&'); }
        ,   "gt" => { return Some('>'); }
        ,   "lt" => { return Some('<'); }
        ,   "quot" => { return Some('"'); }
        ,   "nbsp" => { return Some(' '); }
        ,   _ => {}
        }
    }

    None
}

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

type XmlInputReader<'a> = quick_xml::Reader<&'a [u8]>;

fn parse_xml_sz_and_vbox(
    src_buf: &[u8],
) -> Result<(IVec2, DVec2, DVec2), Box< dyn Error > > {
    let target_tag = "svg".as_bytes();

    let target_attr_key_width = "width".as_bytes();
    let target_attr_key_height = "height".as_bytes();
    let target_attr_key_viewbox = "viewBox".as_bytes();

    let mut sz = IVec2::ZERO;
    let mut viewbox_xy = DVec2::ZERO;
    let mut viewbox_sz = DVec2::ZERO;

    let mut r_src = XmlInputReader::from_reader(&src_buf);

    loop {
        let event = r_src.read_event();

        match event {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(quick_xml::events::Event::Start(ref tag)) => {
                if tag.name().as_ref() == target_tag {
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

fn parse_xml_config(
    src_buf: &[u8],
) -> Result<ImageInfoConfig, Box< dyn Error > > {

    fn func_get_text( r_src: &mut XmlInputReader ) -> Result< String, Box< dyn Error > >
    {
        let target_tag_tspan = "tspan".as_bytes();

        let mut text = String::new();

        loop {
            let event = r_src.read_event();

            match event {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(quick_xml::events::Event::End( tag )) => {
                    if tag.name().as_ref() == target_tag_tspan
                    {
                        text += "\n";
                        break;
                    }
                }
                Ok(quick_xml::events::Event::Start(ref tag)) => {
                    if tag.name().as_ref() == target_tag_tspan
                    {
                        match func_get_text( r_src )
                        {
                            Ok( x ) => { text += &x; },
                            Err( x ) => { return Err( x ); }
                        }
                    }
                },
                Ok(quick_xml::events::Event::Text( inner)) => {
                    text += std::str::from_utf8( inner.as_ref() ).unwrap();
                },
                Ok(quick_xml::events::Event::CData( inner)) => {
                    text += std::str::from_utf8( inner.as_ref() ).unwrap();
                },
                Ok(quick_xml::events::Event::GeneralRef( inner)) => {
                    if let Some( x ) = to_char( &inner )
                    {
                        text.push(  x );
                    }
                },
                Err( x ) => { return Err( Box::new( x ) ); }
                _ => {}
            }
        }

        Ok( text )
    }

    let target_tag = "text".as_bytes();

    let mut text = String::new();

    let mut r_src = XmlInputReader::from_reader(&src_buf);

    loop {
        let event = r_src.read_event();

        match event {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(quick_xml::events::Event::Start(ref tag)) => {
                if tag.name().as_ref() == target_tag
                {
                    match func_get_text( &mut r_src )
                    {
                        Ok( x ) => { text += &x; },
                        Err( x ) => { return Err( x ); }
                    }
                    break;
                }
            },
            _ => {}
        }
    }

    debug!( "config text : {:?}",  text );

    let ret: ImageInfoConfig =
        match toml::from_str( &text )
        {
            Ok( x ) =>
            {
                x
            }
        ,   Err( x ) =>
            {
                error!( "config error. {:?}", x );
                ImageInfoConfig::new()
            }
        }
        ;

    Ok( ret )
}

#[derive(Debug, strum::EnumString, strum::Display)]
enum LayerTarget {
    #[strum(to_string = "base")]
    Base,
    #[strum(to_string = "base_text")]
    BaseText,
    #[strum(to_string = "long_handle")]
    LongHandle,
    #[strum(to_string = "short_handle")]
    ShortHandle,
    #[strum(to_string = "second_handle")]
    SecondHandle,
    #[strum(to_string = "center_circle")]
    CenterCircle,
    #[strum(to_string = "sub_second_base")]
    SubSecondBase,
    #[strum(to_string = "sub_second_handle")]
    SubSecondHandle,
    #[strum(to_string = "sub_second_center_circle")]
    SubSecondCenterCircle,
    #[strum(to_string = "config")]
    Config
}

fn parse_xml_center(
    src_buf: &[u8],
    target: LayerTarget
)
-> Result< DVec2, Box< dyn Error > > {

    let target_attr_key_transform = "transform".as_bytes();

    let target_tag = "g".as_bytes();
    let target_attr_key_groupmode = "inkscape:groupmode".as_bytes();
    let target_attr_val_groupmode = "layer".as_bytes();
    let target_attr_key_label = "inkscape:label".as_bytes();

    let target_tag_ellipse = "ellipse".as_bytes();
    let target_tag_circle = "circle".as_bytes();
    let target_attr_key_cx = "cx".as_bytes();
    let target_attr_key_cy = "cy".as_bytes();

    let mut ret = DVec2::ZERO;

    let mut translate_affines: Vec<DAffine2> = Vec::new();

    let mut target_layer = false;

    let get_transform_affine = |tag: &BytesStart<'_>| {
        if let Ok(attr) = tag.try_get_attribute(target_attr_key_transform)
            && let Some(attr) = attr
        {
            if let Ok(attr_transform) = std::str::from_utf8(attr.value.as_ref()) {
                // debug!("attr_translate:{:?}", attr_transform);
                if let Some(x) = parse_svg_transform_value(attr_transform) {
                    return x;
                }
            }
        }

        DAffine2::IDENTITY
    };

    let mut r_src = XmlInputReader::from_reader(&src_buf);

    loop {
        let event = r_src.read_event();

        match event {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(evt) => match evt {
                quick_xml::events::Event::Start(ref tag) => {
                    translate_affines.push(get_transform_affine(tag));

                    // check <g>

                    if translate_affines.len() == 2
                        && tag.name().as_ref() == target_tag
                    {
                        target_layer = false;

                        if let Ok(attr) = tag.try_get_attribute(target_attr_key_groupmode)
                            && let Some(attr) = attr
                        {
                            if attr.value.as_ref() == target_attr_val_groupmode
                            {
                                if let Ok(attr) = tag.try_get_attribute(target_attr_key_label)
                                    && let Some(attr) = attr
                                {
                                    if attr.value.as_ref() == target.to_string().as_bytes()
                                    {
                                        target_layer = true;
                                    }
                                }
                            }
                        }
                    }
                }
                quick_xml::events::Event::Empty(ref tag) => {
                    /*
                    debug!("target_layer: {:?} ", target_layer);
                    debug!("depth: {:?} ", translate_affines.len());
                    debug!("tag: {:?} ", tag);
                    */

                    if target_layer
                        && translate_affines.len() == 2
                        && (
                                tag.name().as_ref() == target_tag_ellipse
                            ||  tag.name().as_ref() == target_tag_circle
                        )
                    {
                        let mut tran_affine = DAffine2::IDENTITY;
                        // debug!("tran_affine A: {:?}", tran_affine);

                        for x in &translate_affines {
                            tran_affine *= x;
                            // debug!("tran_affine B: {:?}", tran_affine);
                        }

                        tran_affine *= get_transform_affine(tag);
                        //debug!("tran_affine C: {:?}", tran_affine);

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

                        ret = tran_affine.transform_point2(vec2);
                        // debug!("ret: {:?}", ret);
                    }
                }
                quick_xml::events::Event::End(ref _tag) => {
                    translate_affines.pop();
                }
                _ => {}
            },
            Err( e) => { return Err( Box::new( e ) ) },
        }
    }

    Ok(ret)
}

fn filter_xml(
    src_buf: &[u8],
    target: LayerTarget,
) -> Result< Vec<u8>, Box< dyn Error > >
{
    let target_tag = "g".as_bytes();
    let target_attr_key_groupmode = "inkscape:groupmode".as_bytes();
    let target_attr_val_groupmode = "layer".as_bytes();
    let target_attr_key_label = "inkscape:label".as_bytes();

    let mut writer = quick_xml::Writer::new(Cursor::new(Vec::<u8>::new()));

    let mut depth = 0;
    let mut depth_dis_output = -1;

    let mut r_src = XmlInputReader::from_reader(&src_buf);

    loop {
        let event = r_src.read_event();

        match event {
            Ok(quick_xml::events::Event::Eof) => break,

            Ok(evt) => match evt {
                quick_xml::events::Event::Start(ref tag) => {
                    depth += 1;

                    if depth == 2 && tag.name().as_ref() == target_tag {
                        let mut output = false;

                        if let Ok(attr) = tag.try_get_attribute(target_attr_key_groupmode)
                            && let Some(attr) = attr
                        {
                            if attr.value.as_ref() == target_attr_val_groupmode
                            {
                                if let Ok(attr) = tag.try_get_attribute(target_attr_key_label)
                                    && let Some(attr) = attr
                                {
                                    if attr.value.as_ref() == target.to_string().as_bytes() {
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
                quick_xml::events::Event::End(ref _tag) => {
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

            Err( e) => { return Err( Box::new( e ) ) },
        }
    }

    Ok(writer.into_inner().into_inner())
}

#[derive(Debug, Serialize, Deserialize)]
struct ImageInfoConfig
{
    theme_name: Option< String >,
    theme_description: Option< String >,
    with_sub_second: Option< bool >, // = true
    with_second: Option< bool >, // = false
    with_text_time_zone: Option< bool >, // = false
    with_text_date: Option< bool >, // = false
    with_text_time: Option< bool >, // = false
    with_text_segment: Option< bool >, // = false
    enable_rotate_center_circle: Option< bool >, // = false
    enable_update_region_every_time: Option< bool >, // = true
}

impl ImageInfoConfig
{
    const fn new() -> Self
    {
        Self
        {
            theme_name: None,
            theme_description: None,
            with_second: None, // = true
            with_sub_second: None, // = true
            with_text_time_zone: None, // = false
            with_text_date: None, // = false
            with_text_time: None, // = false
            with_text_segment: None, // = false
            enable_rotate_center_circle: None, // = false
            enable_update_region_every_time: None, // = false
        }
    }

    fn update_default( &mut self )
    {
        if self.with_second.is_none()
        {
            self.with_second = Some( true );
        }

        if self.with_sub_second.is_none()
        {
            self.with_sub_second = Some( true );
        }

        if self.with_text_time_zone.is_none()
        {
            self.with_text_time_zone = Some( false );
        }

        if self.with_text_date.is_none()
        {
            self.with_text_date = Some( false );
        }

        if self.with_text_time.is_none()
        {
            self.with_text_time = Some( false );
        }

        if self.with_text_segment.is_none()
        {
            self.with_text_segment = Some( false );
        }

        if self.enable_rotate_center_circle.is_none()
        {
            self.enable_rotate_center_circle = Some( false );
        }

        if self.enable_update_region_every_time.is_none()
        {
            self.enable_update_region_every_time = Some( false );
        }
    }
}

struct ImageInfo {
    sz: IVec2,
    viewbox_xy: DVec2,
    viewbox_sz: DVec2,

    bytes_base: Option<Vec<u8>>,
    bytes_base_text: Option<Vec<u8>>,
    bytes_long_handle: Option<Vec<u8>>,
    bytes_short_handle: Option<Vec<u8>>,
    bytes_second_handle: Option<Vec<u8>>,
    bytes_center_circle: Option<Vec<u8>>,
    bytes_sub_second_base: Option<Vec<u8>>,
    bytes_sub_second_handle: Option<Vec<u8>>,
    bytes_sub_second_center_circle: Option<Vec<u8>>,

    svgh_base: Option<SvgHandle>,
    svgh_long_handle: Option<SvgHandle>,
    svgh_short_handle: Option<SvgHandle>,
    svgh_second_handle: Option<SvgHandle>,
    svgh_center_circle: Option<SvgHandle>,
    svgh_sub_second_base: Option<SvgHandle>,
    svgh_sub_second_handle: Option<SvgHandle>,
    svgh_sub_second_center_circle: Option<SvgHandle>,

    center: DVec2,
    center_sub_second: DVec2,

    config: ImageInfoConfig
}

impl ImageInfo {
    const fn new() -> Self {
        Self {
            sz: IVec2::ZERO,
            viewbox_xy: DVec2::ZERO,
            viewbox_sz: DVec2::ZERO,

            bytes_base: None,
            bytes_base_text: None,
            bytes_long_handle: None,
            bytes_short_handle: None,
            bytes_second_handle: None,
            bytes_center_circle: None,
            bytes_sub_second_base: None,
            bytes_sub_second_handle: None,
            bytes_sub_second_center_circle: None,

            svgh_base: None,
            svgh_long_handle: None,
            svgh_short_handle: None,
            svgh_second_handle: None,
            svgh_center_circle: None,
            svgh_sub_second_base: None,
            svgh_sub_second_handle: None,
            svgh_sub_second_center_circle: None,

            center: DVec2::ZERO,
            center_sub_second: DVec2::ZERO,

            config: ImageInfoConfig::new(),
        }
    }
}

fn load_theme( theme: AppInfoTheme, theme_custom: Option< String > ) -> Option< ImageInfo >
{
    let src_buf: Option< Vec<u8> > =
        match theme
        {
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
            _ =>
            {
                let mut src_buf = Vec::<u8>::new();
                let mut src = File::open(theme.source() ).unwrap();
                src.read_to_end( &mut src_buf ).unwrap();

                Some( src_buf )
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

fn load_xml( src_buf: &Vec<u8> ) -> ImageInfo
{
    let src_base = filter_xml( src_buf,LayerTarget::Base );
    let src_base_text = filter_xml( src_buf,LayerTarget::BaseText );
    let src_long_handle = filter_xml( src_buf,LayerTarget::LongHandle );
    let src_short_handle = filter_xml( src_buf, LayerTarget::ShortHandle );
    let src_second_handle = filter_xml( src_buf, LayerTarget::SecondHandle );
    let src_center_circle = filter_xml( src_buf,LayerTarget::CenterCircle );
    let src_sub_second_base = filter_xml( src_buf,LayerTarget::SubSecondBase );
    let src_sub_second_handle = filter_xml( src_buf,LayerTarget::SubSecondHandle );
    let src_sub_second_center_circle = filter_xml( src_buf,LayerTarget::SubSecondCenterCircle );
    let src_config = filter_xml( src_buf,LayerTarget::Config );

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

    if let Ok( src_xml ) = src_config
    {
        if let Ok( config ) = parse_xml_config( &src_xml )
        {
            debug!( "config load   {:?}", config );

            let mut config = config;
            config.update_default();

            debug!( "config update {:?}", config );

            ret.config = config;
        }
    }

    if let Ok(src_xml) = src_base {
        if let Ok(result) = parse_xml_sz_and_vbox(&src_xml ) {
            ret.sz = result.0;
            ret.viewbox_xy = result.1;
            ret.viewbox_sz = result.2;
        }

        ret.svgh_base = fn_make_svg_handle( &src_xml );
        ret.bytes_base = Some(src_xml);
    }

    if let Ok(src_xml) = src_base_text {
        ret.bytes_base_text = Some(src_xml);
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

        if let Ok(center) = parse_xml_center(&src_xml,LayerTarget::CenterCircle ) {
            ret.center = center;
        }

        debug!("ret.center: {:?}", ret.center);

        ret.svgh_center_circle = fn_make_svg_handle( &src_xml );
        ret.bytes_center_circle = Some(src_xml);
    }

    if let Ok(src_xml) = src_sub_second_base {
        ret.svgh_sub_second_base = fn_make_svg_handle( &src_xml );
        ret.bytes_sub_second_base = Some(src_xml);
    }

    if let Ok(src_xml) = src_sub_second_handle {
        ret.svgh_sub_second_handle = fn_make_svg_handle( &src_xml );
        ret.bytes_sub_second_handle = Some(src_xml);
    }

    if let Ok(src_xml) = src_sub_second_center_circle {

        if let Ok(center) = parse_xml_center(&src_xml,LayerTarget::SubSecondCenterCircle ) {
            ret.center_sub_second = center;
        }

        debug!("ret.center: {:?}", ret.center);

        ret.svgh_sub_second_center_circle = fn_make_svg_handle( &src_xml );
        ret.bytes_sub_second_center_circle = Some(src_xml);
    }

    ret
}

fn load_logo() -> Option< Pixbuf >
{
    // load logo
    let mut src_buf = Vec::<u8>::new();

    let file = "logo.png";

    let mut src = File::open(file ).unwrap();
    src.read_to_end(&mut src_buf).unwrap();

    if file.ends_with( ".svg" )
    {
        if let Ok(result) = parse_xml_sz_and_vbox(&src_buf)
        {

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
    }
    else if file.ends_with( ".png" )
    {
        let mut src_buf = &src_buf[..];  // with io.Read Trait

        let surface = ImageSurface::create_from_png(  &mut src_buf ).unwrap();

        return gdk::pixbuf_get_from_surface( &surface, 0, 0, surface.width(), surface.height() );
    }

    None
}

#[derive(Debug, PartialEq, strum::EnumString, strum::Display, strum::EnumIter, Copy, Clone, Serialize, Deserialize )]
enum AppInfoTheme
{
    Theme1
,   Theme2
,   Theme3
,   Theme4
,   Theme5
,   Theme6
,   Custom
}

impl AppInfoTheme
{
    fn name( &self ) -> String
    {
        match self
        {
            Self::Theme1 => format!( "[{}] {}", self.to_string(), "Classic, With sub second, without text." )
        ,   Self::Theme2 => format!( "[{}] {}", self.to_string(), "Modern, Square, With everything." )
        ,   Self::Theme3 => format!( "[{}] {}", self.to_string(), "Modern, With digital segment." )
        ,   _ => format!( "[{}]", self.to_string() )
        }
    }

    fn source( &self ) -> &str
    {
        match self
        {
            Self::Theme1 => "clock_theme_1.svg"
        ,   Self::Theme2 => "clock_theme_2.svg"
        ,   Self::Theme3 => "clock_theme_3.svg"
        ,   Self::Theme4 => "clock_theme_4.svg"
        ,   Self::Theme5 => "clock_theme_5.svg"
        ,   Self::Theme6 => "clock_theme_6.svg"
        ,   _ => ""
        }
    }
}

#[derive(Debug, PartialEq, strum::EnumString, strum::Display, strum::EnumIter, Copy, Clone, Serialize, Deserialize )]
enum AppInfoFormatDate
{
    Fmt1
,   Fmt2
,   Fmt3
,   Fmt4
,   Custom
}

impl AppInfoFormatDate
{
    fn format_str( &self ) -> ( &str, &str )
    {
        // see https://docs.rs/chrono/latest/chrono/format/strftime/index.html

        match self
        {
            Self::Fmt1 =>
                (
                    "%F" /* Year-month-day format (ISO 8601). Same as %Y-%m-%d. */
                ,   "Year-month-day (e.g., 2025-10-31)"
                )
        ,   Self::Fmt2 =>
                (
                    "%Y/%m/%d" /* Year/month/day format. Same as %Y/%m/%d. */
                ,   "Year/month/day (e.g., 2025/10/31)"
                )
        ,   Self::Fmt3 =>
                (
                    "%D" /* Month/day/year format. Same as %m/%d/%y. */
                ,   "Month/day/year (e.g., 10/31/25)"
                )
        ,   Self::Fmt4 =>
                (
                    "%v" /* Day-month-year format. Same as %e-%b-%Y. */
                ,   "Day-month-year (e.g., 31-Oct-2025)"
                )
        ,   _ => ( "", "Custom" )
        }
    }
}

#[derive(Debug, PartialEq, strum::EnumString, strum::Display, strum::EnumIter, Copy, Clone, Serialize, Deserialize )]
enum AppInfoFormatTime
{
    Fmt1
,   Fmt2
,   Fmt3
,   Fmt4
,   Fmt5
,   Custom
}

impl AppInfoFormatTime
{
    fn format_str( &self ) -> ( &str, &str )
    {
        // see https://docs.rs/chrono/latest/chrono/format/strftime/index.html

        match self
        {
            Self::Fmt1 =>
                (
                    "%r" /* Locale’s 12 hour clock time. (e.g., 11:11:04 PM). Falls back to %X if the locale does not have a 12 hour clock format. */
                ,   "12 hour clock time with AM/PM (e.g., 11:11:04 PM)"
                )
        ,   Self::Fmt2 =>
                (
                    "%I:%M:%S" /* Locale’s 12 hour clock time. (e.g., 11:11:04). Falls back to %X if the locale does not have a 12 hour clock format. */
                ,   "12 hour clock time without AM/PM (e.g., 11:11:04)"
                )
        ,   Self::Fmt3 =>
                (
                    "%T" /* Hour-minute-second format. Same as %H:%M:%S. */
                ,   "Hour-minute-second (e.g., 23:11:04) "
                )
        ,   Self::Fmt4 =>
                (
                    "%R" /* Hour-minute format. Same as %H:%M. */
                ,   "Hour-minute (e.g., 23:11)"
                )
        ,   Self::Fmt5 =>
                (
                    "%p"
                ,   "AM/PM (e.g., AM)"
                )
        ,   _ => ( "", "Custom" )
        }
    }
}

/*
const FORMAT_TIME_HMS: &str = "%H:%M:%S";
const FORMAT_TIME_AMPM: &str = "%p";
*/

#[derive(Debug, Serialize, Deserialize)]
struct AppInfo
{
    always_on_top: bool
,   lock_pos: bool
,   show_seconds: bool
,   enable_sub_second_hand: bool
,   enable_second_hand_smoothly: bool
,   enable_text_time_zone: bool
,   enable_text_date: bool
,   enable_text_time: bool
,   enable_text_time_segment: bool
,   enable_text_time_segment_hour12: bool
,   enable_text_time_segment_dotblink: bool
,   text_format_date: AppInfoFormatDate
,   text_format_date_custom: Option< String >
,   text_format_time: AppInfoFormatTime
,   text_format_time_custom: Option< String >
,   time_zone: String
,   theme: AppInfoTheme
,   theme_custom: Option< String >
,   zoom: u32
,   window_pos: Option< ( i32, i32 ) >
,   #[serde(skip)]
    zoom_update: bool
,   #[serde(skip)]
    time_disp: NaiveDateTime
,   #[serde(skip)]
    time_disp_st: Option< ( NaiveDateTime, DateTime<Utc> ) >
,   #[serde(skip)]
    timer_sourceid: RefCell< Option< gtk::glib::SourceId > >
,   #[serde(skip)]
    time_disp_force: Option< NaiveTime >
}

impl AppInfo
{
    const fn new() -> Self {

        Self
        {
            always_on_top: true
        ,   lock_pos: false
        ,   show_seconds: true
        ,   enable_sub_second_hand: false
        ,   enable_second_hand_smoothly: true
        ,   enable_text_time_zone: true
        ,   enable_text_date: true
        ,   enable_text_time: true
        ,   enable_text_time_segment: true
        ,   enable_text_time_segment_hour12: false
        ,   enable_text_time_segment_dotblink: true
        ,   text_format_date: AppInfoFormatDate::Fmt1
        ,   text_format_date_custom: None
        ,   text_format_time: AppInfoFormatTime::Fmt1
        ,   text_format_time_custom: None
        ,   time_zone: String::new()
        ,   theme: AppInfoTheme::Theme1
        ,   theme_custom: None
        ,   zoom: 100
        ,   window_pos: None

        ,   zoom_update: true
        ,   time_disp: DateTime::UNIX_EPOCH.naive_utc()
        ,   time_disp_st: None
        ,   timer_sourceid: RefCell::new( None )
        ,   time_disp_force: None
        }
    }

    fn reset( &mut self )
    {
        self.zoom_update = true;
        self.time_disp = Local::now().date_naive().and_hms_opt(0,0,0).unwrap();
        self.time_disp_st = None;

        self.theme_custom =
            if let Ok( x ) = std::env::var( "THEME_CUSTOM" )
            {
                Some( x )
            }
            else
            {
                None
            }
            ;

        self.time_disp_force =
            if let Ok( x ) = std::env::var( "FIX_TIME" )
            {
                if let Ok( x ) = NaiveTime::parse_from_str( &x, "%H:%M:%S")
                {
                    Some( x )
                }
                else if let Ok( x ) = NaiveTime::parse_from_str( &x, "%H:%M")
                {
                    Some( x )
                }
                else
                {
                    None
                }
            }
            else
            {
                None
            }
            ;
    }
}

const MOVE_FAST_SECS: i64 = 5;

fn update_watch( da: &DrawingArea, app_info: &mut AppInfo )
{
    let time_now = Local::now();

    let time_now_naive =
        if app_info.time_disp_force.is_some()
        {
            time_now.with_time( app_info.time_disp_force.as_ref().copied().unwrap() ).unwrap().naive_local()
        }
        else if app_info.time_zone == ""
        {
            time_now.naive_local()
        }
        else
        {
            let time_zone = app_info.time_zone.clone();

            if time_zone.starts_with( "GMT+" ) || time_zone.starts_with( "GMT-" )
            {
                // FIX
                // chrono::Tz::Etc__GMTMinus1 = +1 -> -1
                // chrono::Tz::Etc__GMTPlus1  = -1 -> +1
                // chrono::Tz::Etc__GMTMinus<x> = +<x> -> -<x>
                // chrono::Tz::Etc__GMTPlus<x>  = -<x> -> +<x>
                // see https://github.com/chronotope/chrono-tz/issues/16
                // see https://github.com/eggert/tz/blob/ab21ad9710b88f28995b7ed47c6efda47ffb1be5/etcetera#L37-L43
                // see ```
                // # Be consistent with POSIX TZ settings in the Zone names,
                // # even though this is the opposite of what many people expect.
                // # POSIX has positive signs west of Greenwich, but many people expect
                // # positive signs east of Greenwich.  For example, TZ='Etc/GMT+4' uses
                // # the abbreviation "-04" and corresponds to 4 hours behind UT
                // # (i.e. west of Greenwich) even though many people would expect it to
                // # mean 4 hours ahead of UT (i.e. east of Greenwich).
                // ```

                if let Ok( time_delta ) = i32::from_str( time_zone.trim_start_matches( "GMT" ) )
                {
                    let offset = chrono::FixedOffset::east_opt( time_delta * 60 * 60 ).unwrap();
                    time_now.with_timezone( &offset ).naive_local()
                }
                else
                {
                    time_now.naive_local()
                }
            }
            else
            {
                let tz: Result< chrono_tz::Tz, _> = time_zone.parse();

                match tz
                {
                    Ok( offset ) =>
                    {
                        time_now.with_timezone( &offset ).naive_local()
                    }
                ,    _ =>
                    {
                        time_now.naive_local()
                    }
                }
            }
       }
       ;

    let has_time_disp_st = app_info.time_disp_st.is_some();

    let time_delta = ( time_now_naive - app_info.time_disp ).num_seconds();

    if time_delta.abs() <= 10
    {
        app_info.time_disp = time_now_naive;
        app_info.time_disp_st = None;
    }
    else
    {
        if app_info.time_disp_st.is_none()
        {
            app_info.time_disp_st = Some( ( app_info.time_disp, time_now.to_utc() ) );
        }

        let ( time_disp_st, time_st ) = app_info.time_disp_st.unwrap();

        let timestamp_disp_du = TimeDelta::seconds( MOVE_FAST_SECS );

        let mut tweener = tween::Tweener::quad_in_out( 0.0, 1.0, timestamp_disp_du.num_milliseconds()  );

        let timestamp_disp_diff = time_now.to_utc() - time_st;

        let pos = tweener.move_to( timestamp_disp_diff.num_milliseconds() );

        if pos < 0.0 || pos >= 1.0
        {
            app_info.time_disp = time_now_naive;
            app_info.time_disp_st = None;
        }
        else
        {
            let add=  TimeDelta::milliseconds( ( ( time_now_naive - time_disp_st ).num_milliseconds() as f64 * pos ) as i64 );
            app_info.time_disp = time_disp_st + add;
        }
    }

    if has_time_disp_st != app_info.time_disp_st.is_some()
    {
        // update timer

        let is_fast = app_info.time_disp_st.is_some();

        {
            let da = da.clone();

            let old_timer_sourceid = app_info.timer_sourceid.replace(
                Some(
                    gtk::glib::source::timeout_add_local(
                        std::time::Duration::from_millis( get_timer_interval( is_fast ) ),
                        move ||
                        {
                            da.queue_draw();
                            gtk::glib::ControlFlow::Continue
                        }
                    )
                )
            );

            if let Some( sourceid ) = old_timer_sourceid
            {
                sourceid.remove();
            }
        }
    }
}

fn update_region<'a>( window: &'a ApplicationWindow, image_info: &'a ImageInfo, app_info: &'a mut AppInfo )
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

        if app_info.zoom_update
        {
            window.resize( sz.x as i32, sz.y as i32 );
            window.shape_combine_region( make_region( &image_info, app_info ).as_ref() );
            app_info.zoom_update = false;
        }
        else if let Some( x ) = image_info.config.enable_update_region_every_time && x
        {
            window.shape_combine_region( make_region( &image_info, app_info ).as_ref() );
        }
    }
}


fn make_region( image_info : &ImageInfo, app_info: &AppInfo ) -> Option< Region >
{
    let zoom_factor = app_info.zoom as f64 / 100.0;

    let sz = DVec2::new(
        image_info.sz.x as f64 * zoom_factor
    ,   image_info.sz.y as f64 * zoom_factor
    );

    if     image_info.sz.x > 0
        && image_info.sz.y > 0
        && sz.x > 0.0
        && sz.y > 0.0
    {
        let surface_mask = ImageSurface::create(Format::A8, sz.x as i32, sz.y as i32 ).unwrap();
        let cctx = Context::new(&surface_mask).unwrap();

        draw_watch( &cctx, image_info, app_info, true );

        surface_mask.create_region()
    }
    else
    {
        None
    }
}

fn draw_watch(
    cctx : &Context, image_info : &ImageInfo, app_info: &AppInfo, for_region: bool )
{
    let zoom_factor = app_info.zoom as f64 / 100.0;

    let sz = DVec2::new(
        image_info.sz.x as f64 * zoom_factor
    ,   image_info.sz.y as f64 * zoom_factor
    );

    let viewport = Rectangle::new(0.0, 0.0, sz.x, sz.y );

    let func_render = | svg_handle : &SvgHandle |
    {
        let svg_renderer = rsvg::CairoRenderer::new(svg_handle);
        svg_renderer.render_document(cctx, &viewport).unwrap();
    };

    let center = DVec2
    {
        x: sz.x * ( image_info.center.x / image_info.viewbox_sz.x )
    ,   y: sz.y * ( image_info.center.y / image_info.viewbox_sz.y )
    };

    let center_sub_second = DVec2
    {
        x: sz.x * ( image_info.center_sub_second.x / image_info.viewbox_sz.x )
    ,   y: sz.y * ( image_info.center_sub_second.y / image_info.viewbox_sz.y )
    };

    let func_render_rotate = | svg_handle : &SvgHandle, _center: &DVec2, angle : f64 |
    {
        let _ = cctx.save();

        cctx.translate( _center.x * 1.0, _center.y * 1.0 );
        cctx.rotate( angle * ( PI / 180.0 ) );
        cctx.translate( _center.x * -1.0, _center.y * -1.0 );

        let svg_renderer = rsvg::CairoRenderer::new( svg_handle );
        svg_renderer.render_document(cctx, &viewport).unwrap();

        let _ = cctx.restore();
    };

    let time_now =
        if app_info.time_disp_force.is_some()
        {
            //DateTime::UNIX_EPOCH.with_timezone( &Local )
            Local::now().with_time( app_info.time_disp_force.unwrap() ).unwrap()
        }
        else
        {
            Local::now()
        }
        ;

    let time_secs = app_info.time_disp.hour12().1 * 60 * 60 + app_info.time_disp.minute() * 60 + app_info.time_disp.second();

    let angle_hour = time_secs as f64 / ( 12.0 * 60.0 * 60.0 ) * 360.0;
    let angle_min = time_secs as f64 / ( 60.0 * 60.0 ) * 360.0;

    let angle_sec_delta = if app_info.enable_second_hand_smoothly { time_now.timestamp_subsec_millis() as f64 / 1000.0 } else { 0.0 };
    let angle_sec = ( time_now.second() as f64 + angle_sec_delta ) / 60.0 * 360.0;

    // paint base BLACK ( for not region )
    if !for_region
    {
        cctx.rectangle( viewport.x(), viewport.y(), viewport.width(), viewport.height() );
        cctx.set_source_rgb(0.0, 0.0, 0.0);
        let _ = cctx.fill();
    }

    // render base
    if let Some( svgh ) = image_info.svgh_base.as_ref()
    {
        let svg_renderer = rsvg::CairoRenderer::new(svgh);
        svg_renderer.render_document(cctx, &viewport).unwrap();
    }

    // render sub_base_text

    let with_text_time_zone = if let Some( x ) = image_info.config.with_text_time_zone && x { true } else { false };
    let with_text_date = if let Some( x ) = image_info.config.with_text_date && x { true } else { false };
    let with_text_time = if let Some( x ) = image_info.config.with_text_time && x { true } else { false };
    let with_text_segment = if let Some( x ) = image_info.config.with_text_segment && x { true } else { false };

    if ( with_text_time_zone || with_text_date || with_text_time || with_text_segment )
    && let Some( x ) = image_info.bytes_base_text.as_ref()
    {
        // text replace
        static RE_REPLACE: LazyLock<regex::bytes::Regex> = LazyLock::new(
            ||
            {
                regex::bytes::Regex::new(r"(?i)\{\{\s*([A-Za-z0-9_]+)\s*\}\}").unwrap()
            }
        );

        static RE_SEGMENT_NUM: LazyLock<regex::bytes::Regex> = LazyLock::new(
            ||
            {
                regex::bytes::Regex::new(r"(?i)seg_(hh|hl|mh|ml|sh|sl)([a-g])").unwrap()
            }
        );

        static RE_SEGMENT_AMPM: LazyLock<regex::bytes::Regex> = LazyLock::new(
            ||
            {
                regex::bytes::Regex::new(r"(?i)seg_(amb|pmb|am|pm)").unwrap()
            }
        );

        static RE_SEGMENT_DOT: LazyLock<regex::bytes::Regex> = LazyLock::new(
            ||
            {
                regex::bytes::Regex::new(r"(?i)seg_dot").unwrap()
            }
        );

        let mut buf: Vec<u8> = Vec::new();
        let mut last_match = 0;

        for caps in RE_REPLACE.captures_iter( x )
        {
            let m0 = caps.get(0).unwrap();
            let m1 = caps.get(1).unwrap();

            buf.extend_from_slice( &x[last_match..m0.start()] );

            let kw = m1.as_bytes().to_ascii_lowercase();

            match kw.as_slice()
            {
                /* https://docs.rs/chrono/latest/chrono/format/strftime/index.html */

                b"time_zone" =>
                {
                    if with_text_time_zone && app_info.enable_text_time_zone
                    {
                        buf.extend_from_slice( app_info.time_zone.as_bytes() );
                    }
                }
                b"date" =>
                {
                    if with_text_date && app_info.enable_text_date
                    {
                        if app_info.text_format_date == AppInfoFormatDate::Custom && app_info.text_format_date_custom.is_some()
                        {
                            let df = app_info.time_disp.format( app_info.text_format_date_custom.as_ref().unwrap() );

                            let mut buffer = String::new();

                            if let Ok(_) = df.write_to(&mut buffer)
                            {
                                buf.extend_from_slice( buffer.as_bytes() );
                            }
                        }
                        else
                        {
                            buf.extend_from_slice( app_info.time_disp.format( app_info.text_format_date.format_str().0 ).to_string().as_bytes() );
                        }
                    }
                }
                b"time" =>
                {
                    if with_text_time && app_info.enable_text_time
                    {
                        if app_info.text_format_time == AppInfoFormatTime::Custom && app_info.text_format_time_custom.is_some()
                        {
                            let df = app_info.time_disp.format( app_info.text_format_time_custom.as_ref().unwrap() );

                            let mut buffer = String::new();

                            if let Ok(_) = df.write_to(&mut buffer)
                            {
                                buf.extend_from_slice( buffer.as_bytes() );
                            }
                        }
                        else
                        {
                            buf.extend_from_slice( app_info.time_disp.format( app_info.text_format_time.format_str().0 ).to_string().as_bytes() );
                        }
                    }
                }
                /*
                b"time_hms" =>
                {
                    if app_info.enable_text_time
                    {
                        buf.extend_from_slice( app_info.time_disp.format( FORMAT_TIME_HMS ).to_string().as_bytes() );
                    }
                }
                b"time_ampm" =>
                {
                    if app_info.enable_text_time_ampm
                    {
                        buf.extend_from_slice( app_info.time_disp.format( FORMAT_TIME_AMPM ).to_string().as_bytes() );
                    }
                }
                */
                _ =>
                {
                    if with_text_segment && app_info.enable_text_time_segment
                    {
                        if let Some( caps ) = RE_SEGMENT_NUM.captures( kw.as_slice() )
                        {
                            /*
                                <g visibility="{{seg_(hh|hl|mh|ml|sh|sl)([a-g])}}"></g>
                                ex.
                                <g visibility="{{seg_hha}}"></g>

                                {{seg_xxx}} = "visible" or "hidden"

                                $1 = (hh|hl|mh|ml|sh|sl)
                                    hh -> Hour High digit
                                    hl -> Hour Low digit
                                    mh -> Minute High digit
                                    ml -> Minute Low digit
                                    sh -> Second High digit
                                    sl -> Second Low digit

                                $2 = a,b,c,d,e,f,g
                                    segment https://en.wikipedia.org/wiki/Seven-segment_display#/media/File:7_Segment_Display_with_Labeled_Segments.svg
                                       =a=
                                    |f|   |b|
                                       =g=
                                    |e|   |c|
                                       =d=
                            */

                            let m1 = caps.get(1).unwrap();
                            let m2 = caps.get(2).unwrap();

                            let flag_12 = app_info.enable_text_time_segment_hour12;

                            let num =
                                match m1.as_bytes()
                                {
                                    b"hh" => { ( if flag_12 { app_info.time_disp.hour12().1 } else { app_info.time_disp.hour() } ) / 10 }
                                ,   b"hl" => { ( if flag_12 { app_info.time_disp.hour12().1 } else { app_info.time_disp.hour() } ) % 10 }
                                ,   b"mh" => { app_info.time_disp.minute() / 10 }
                                ,   b"ml" => { app_info.time_disp.minute() % 10 }
                                ,   b"sh" => { app_info.time_disp.second() / 10 }
                                ,   b"sl" => { app_info.time_disp.second() % 10 }
                                ,   _ => { 0 }
                                };

                            // num -> seg
                            // https://ja.wikipedia.org/wiki/7%E3%82%BB%E3%82%B0%E3%83%A1%E3%83%B3%E3%83%88%E3%83%87%E3%82%A3%E3%82%B9%E3%83%97%E3%83%AC%E3%82%A4#%E6%95%B0%E3%81%8B%E3%82%897%E3%82%BB%E3%82%B0%E3%83%A1%E3%83%B3%E3%83%88%E3%82%B3%E3%83%BC%E3%83%89%E3%81%B8%E3%81%AE%E5%A4%89%E6%8F%9B

                            let b_on: u8 =
                                match num
                                {
                                    0 => { 0x7E }
                                ,   1 => { 0x30 }
                                ,   2 => { 0x6d }
                                ,   3 => { 0x79 }
                                ,   4 => { 0x33 }
                                ,   5 => { 0x5b }
                                ,   6 => { 0x5f }
                                ,   7 => { 0x70 }
                                ,   8 => { 0x7f }
                                ,   9 => { 0x7b }
                                ,   _ => { 0x00 }
                                };

                            let b_mask: u8 =
                                match m2.as_bytes()
                                {
                                    b"a" => { 0x1 << 6 }
                                ,   b"b" => { 0x1 << 5 }
                                ,   b"c" => { 0x1 << 4 }
                                ,   b"d" => { 0x1 << 3 }
                                ,   b"e" => { 0x1 << 2 }
                                ,   b"f" => { 0x1 << 1 }
                                ,   b"g" => { 0x1 << 0 }
                                ,   _ => { 0x0u8 }
                                };

                            buf.extend_from_slice(
                                if b_on & b_mask != 0x00
                                {
                                    b"visible"
                                }
                                else
                                {
                                    b"hidden"
                                }
                            );

                            /*
                            if log_enabled!(Level::Debug)
                            {
                                if m1.as_bytes() == b"sl"
                                {
                                    let c = String::from_utf8( m2.as_bytes().to_vec() );

                                    debug!("{}:{:?}:{:02X}:{:02X}:{}", num, c, b_on, b_mask, b_on & b_mask );
                                }
                            }
                            */
                        }
                        else if let Some( caps ) = RE_SEGMENT_AMPM.captures( kw.as_slice() )
                        {
                            /*
                                <g visibility="{{seg_am}}"></g>
                                <g visibility="{{seg_pm}}"></g>
                                <g visibility="{{seg_amb}}"></g>
                                <g visibility="{{seg_pmb}}"></g>

                                {{seg_am}}, {{seg_pm}}, {{seg_amb}}, {{seg_pmb}} = "visible" or "hidden"
                            */
                            let is_enable = app_info.enable_text_time_segment_hour12;
                            let is_pm = app_info.time_disp.hour() >= 12;

                            let m1 = caps.get(1).unwrap();

                            // debug!("{:?}", String::from_utf8( m1.as_bytes().to_vec() ) );

                            buf.extend_from_slice(
                                if  is_enable &&
                                    (
                                        !is_pm && m1.as_bytes() == b"am"
                                    ||  is_pm && m1.as_bytes() == b"pm"
                                    ||  m1.as_bytes() == b"amb"
                                    ||  m1.as_bytes() == b"pmb"
                                    )
                                {
                                    b"visible"
                                }
                                else
                                {
                                    b"hidden"
                                }
                            );
                        }
                        else if let Some( _ ) = RE_SEGMENT_DOT.captures( kw.as_slice() )
                        {
                            /*
                                <g visibility="{{seg_dot}}"></g>

                                {{seg_dot}} = "visible" or "hidden"
                            */

                            let is_on =
                                if app_info.enable_text_time_segment_dotblink
                                {
                                    app_info.time_disp.and_utc().timestamp_subsec_millis() < 500
                                }
                                else
                                {
                                    true
                                }
                                ;

                            buf.extend_from_slice( if is_on { b"visible" } else { b"hidden" } );
                        }
                    }
                }
            }

            last_match = m0.end();
        }

        buf.extend_from_slice( &x[last_match..] );

        // render
        let svg_stream = gtk::gio::MemoryInputStream::from_bytes(&gtk::glib::Bytes::from( &buf ));

        if let Ok( svgh ) =
            rsvg::Loader::new()
            .read_stream(
                &svg_stream,
                None::<&gtk::gio::File>,
                None::<&gtk::gio::Cancellable>,
            )
        {
            let svg_renderer = rsvg::CairoRenderer::new( &svgh );
            svg_renderer.render_document(cctx, &viewport).unwrap();
        }
    }

    // render sub_second
    if      app_info.show_seconds
        &&  app_info.enable_sub_second_hand
        &&  let Some(x) = image_info.config.with_sub_second
        &&  x
    {
        // render sub_second_base
        if let Some( svgh ) = image_info.svgh_sub_second_base.as_ref()
        {
            let svg_renderer = rsvg::CairoRenderer::new(svgh);
            svg_renderer.render_document(cctx, &viewport).unwrap();
        }

        // render sub_second_handle
        if let Some( svgh ) = image_info.svgh_sub_second_handle.as_ref()
        {
            func_render_rotate( svgh, &center_sub_second, angle_sec );
        }

        // render sub_second_center_circle
        if let Some( svgh ) = image_info.svgh_sub_second_center_circle.as_ref()
        {
            if let Some( x ) = image_info.config.enable_rotate_center_circle && x
            {
                func_render_rotate( svgh, &center_sub_second, angle_sec );
            }
            else
            {
                func_render( svgh );
            }
        }
    }

    // render long_handle
    if let Some( svgh ) = image_info.svgh_long_handle.as_ref()
    {
        func_render_rotate( svgh, &center, angle_min );
    }

    // render short_handle
    if let Some( svgh ) = image_info.svgh_short_handle.as_ref()
    {
        func_render_rotate( svgh, &center, angle_hour );
    }

    // render second_handle
    if      app_info.show_seconds
        && !app_info.enable_sub_second_hand
        &&  let Some( x ) = image_info.config.with_second
        &&  x
    {
        if let Some( svgh ) = image_info.svgh_second_handle.as_ref()
        {
            func_render_rotate( svgh, &center, angle_sec );
        }
    }

    // render center_circle
    if let Some( svgh ) = image_info.svgh_center_circle.as_ref()
    {
        if let Some( x ) = image_info.config.enable_rotate_center_circle && x
        {
            func_render_rotate( svgh, &center, angle_sec );
        }
        else
        {
            func_render( svgh );
        }
    }

}

fn make_png_image(
    image_info : &ImageInfo, app_info: &mut AppInfo )
{
    let zoom_factor = app_info.zoom as f64 / 100.0;

    let sz = DVec2::new(
        image_info.sz.x as f64 * zoom_factor
    ,   image_info.sz.y as f64 * zoom_factor
    );

    let mut surface = ImageSurface::create(Format::ARgb32, sz.x as i32, sz.y as i32).unwrap();

    {
        let cctx = Context::new(&surface).unwrap();
        draw_watch( &cctx, image_info, app_info, false );
        surface.flush();
    }

    if  let Some( region ) = make_region( image_info, app_info )
    {
        let h = surface.height();
        let w = surface.width();
        let s = surface.stride();

        match surface.data()
        {
            Ok( mut data ) =>
            {
                for y in 0..h
                {
                    for x in 0..w
                    {
                        if !region.contains_point(x, y)
                        {
                            let p = y * s + x * 4;
                            data[ ( p + 0 ) as usize] = 0x00; // b
                            data[ ( p + 1 ) as usize] = 0x00; // g
                            data[ ( p + 2 ) as usize] = 0x00; // r
                            data[ ( p + 3 ) as usize] = 0x00; // a
                        }
                    }
                }
            }
        ,   Err( x ) =>
            {
                debug!("{:?}", x )
            }
        }

        surface.mark_dirty();
    }

    let mut screenshot_file = File::create("ss.png").unwrap();
    let _ = surface.write_to_png(&mut screenshot_file);
}

fn make_theme_menu(
    image_info: &Rc< RefCell< ImageInfo > >,
    app_info: &Rc< RefCell< AppInfo > >
) -> Menu
{
    let menu = Menu::new();

    for ait in AppInfoTheme::iter()
    {
        let menu_item = CheckMenuItem::with_label( ait.name().as_str() );

        menu_item.set_active( ait == app_info.borrow().theme );

        {
            let app_info = app_info.clone();
            let image_info = image_info.clone();

            menu_item.connect_activate(
                move |_|
                {
                    let mut app_info = app_info.borrow_mut();
                    app_info.theme = ait;
                    app_info.zoom = 100;
                    app_info.zoom_update = true;
                    image_info.replace( load_theme( app_info.theme, app_info.theme_custom.clone() ).unwrap() );
                }
            );
        }

        if ait == AppInfoTheme::Custom
        {
            menu_item.set_sensitive( app_info.borrow().theme_custom.is_some() );
        }

        menu.append( &menu_item );
    }

    menu
}

fn make_zoom_menu(
    app_info: &Rc< RefCell< AppInfo > >
) -> Menu
{
    static ZOOMS: LazyLock< Vec<u32> > = LazyLock::new(
        ||
        {
            (30..=230).step_by(10).collect()
        }
    );

    let zoom = app_info.borrow().zoom;

    if ZOOMS.iter().find( | &&x| { x == zoom } ).is_none()
    {
        app_info.borrow_mut().zoom = 100;
    }

    let menu = Menu::new();

    for &x in ZOOMS.iter()
    {
        let label = format!( "{}%", x );

        let menu_item = CheckMenuItem::with_label( label.as_str() );

        menu_item.set_active( app_info.borrow().zoom == x );

        {
            let app_info = app_info.clone();

            menu_item.connect_activate(
                move |_| {
                    let mut app_info = app_info.borrow_mut();
                    app_info.zoom = x;
                    app_info.zoom_update = true;
                }
            );
        }

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
        let p = n.split_once( "/" );

        if let Some( ( area, city)  ) = p
        {
            if ! dic.contains_key( area )
            {
                dic.insert( area, Vec::<_>::new() );
            }

            if let Some( vec ) = dic.get_mut( area )
            {
                if !( city.starts_with( "GMT" ) || city.starts_with( "UTC" ) || city.starts_with( "UCT" ) )
                {
                    vec.push( city );
                }
            }
        }
    }

    let menu = Menu::new();

    let tz = app_info.borrow().time_zone.clone();
    let tz = if tz == "" { String::from( "<< (Local Time) >>" ) } else { format!("<< {} >>", tz) };

    let menu_item_now = MenuItem::with_label( tz.as_str() );
    menu_item_now.set_sensitive( false );

    menu.append( &menu_item_now );
    menu.append( &SeparatorMenuItem::new() );

    let menu_item_local_time = CheckMenuItem::with_label( "Local Time" );

    menu_item_local_time.set_active( app_info.borrow().time_zone == "" );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_local_time.connect_activate(
            move |_| {
                let mut app_info = app_info.borrow_mut();
                app_info.time_zone = String::from( "" );
                app_info.time_disp_st = None;
                da.queue_draw();
            }
        );
    }

    menu.append( &menu_item_local_time );

    let menu_item_utc = CheckMenuItem::with_label( "UTC" );

    menu_item_utc.set_active( app_info.borrow().time_zone == "UTC" );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_utc.connect_activate(
            move |_| {
                let mut app_info = app_info.borrow_mut();
                app_info.time_zone = String::from( "UTC" );
                da.queue_draw();
            }
        );
    }

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

        {
            let da = da.clone();
            let app_info = app_info.clone();

            menu_item_gmt_entry.connect_activate(
                move |_| {
                    let mut app_info = app_info.borrow_mut();
                    app_info.time_zone = String::from( gmt_entry );
                    da.queue_draw();
                }
            );
        }

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

                {
                    let da = da.clone();
                    let app_info = app_info.clone();
                    menu_item_city.connect_activate(
                        move |_| {
                            let mut app_info = app_info.borrow_mut();
                            app_info.time_zone = tz.clone();
                            da.queue_draw();
                        }
                    );
                }

                menu_area.append( &menu_item_city );
            }

            menu_item_area.set_submenu( Some( &menu_area ) );

            menu.append( &menu_item_area );
        }
    }

    menu
}

fn make_text_visibility_menu(
    da: &DrawingArea,
    app_info: &Rc< RefCell< AppInfo > >,
    image_info: &Rc< RefCell< ImageInfo > >
) -> Menu
{
    let with_time_zone =
        if let Some( x ) = image_info.borrow().config.with_text_time_zone && x { true } else { false }
        ;
    let with_date =
        if let Some( x ) = image_info.borrow().config.with_text_date && x { true } else { false }
        ;
    let with_time =
        if let Some( x ) = image_info.borrow().config.with_text_time && x { true } else { false }
        ;
    let with_segment =
        if let Some( x ) = image_info.borrow().config.with_text_segment && x { true } else { false }
        ;

    let menu = Menu::new();

    let menu_item_enable_text_time_zone = CheckMenuItem::with_label( "Enable Time Zone" );

    menu_item_enable_text_time_zone.set_sensitive( with_time_zone );
    menu_item_enable_text_time_zone.set_active( app_info.borrow().enable_text_time_zone );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_enable_text_time_zone.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_text_time_zone = !app_info.enable_text_time_zone;
                da.queue_draw();
            }
        );
    }

    let menu_item_enable_text_date = CheckMenuItem::with_label( "Enable Date" );

    menu_item_enable_text_date.set_sensitive( with_date );
    menu_item_enable_text_date.set_active( app_info.borrow().enable_text_date );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_enable_text_date.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_text_date = !app_info.enable_text_date;
                da.queue_draw();
            }
        );
    }

    let menu_item_enable_text_time = CheckMenuItem::with_label( "Enable Time" );

    menu_item_enable_text_time.set_sensitive( with_time );
    menu_item_enable_text_time.set_active( app_info.borrow().enable_text_time );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_enable_text_time.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_text_time = !app_info.enable_text_time;
                da.queue_draw();
            }
        );
    }

    /*
    let menu_item_enable_text_time_ampm = CheckMenuItem::with_label( "Enable Time AM/PM" );

    menu_item_enable_text_time_ampm.set_active( app_info.borrow().enable_text_time_ampm );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_enable_text_time_ampm.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_text_time_ampm = !app_info.enable_text_time_ampm;
                da.queue_draw();
            }
        );
    }
    */

    let menu_item_enable_text_time_segment = CheckMenuItem::with_label( "Enable Time Segment" );

    menu_item_enable_text_time_segment.set_sensitive( with_segment );
    menu_item_enable_text_time_segment.set_active( app_info.borrow().enable_text_time_segment );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_enable_text_time_segment.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_text_time_segment = !app_info.enable_text_time_segment;
                da.queue_draw();
            }
        );
    }

    let menu_item_enable_text_time_segment_hour12 = CheckMenuItem::with_label( "Enable Time Segment Hour 12" );

    menu_item_enable_text_time_segment_hour12.set_sensitive( with_segment );
    menu_item_enable_text_time_segment_hour12.set_active( app_info.borrow().enable_text_time_segment_hour12 );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_enable_text_time_segment_hour12.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_text_time_segment_hour12 = !app_info.enable_text_time_segment_hour12;
                da.queue_draw();
            }
        );
    }

    let menu_item_enable_text_time_segment_dotblink = CheckMenuItem::with_label( "Enable Time Segment dot blink" );

    menu_item_enable_text_time_segment_dotblink.set_active( app_info.borrow().enable_text_time_segment_dotblink );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_enable_text_time_segment_dotblink.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_text_time_segment_dotblink = !app_info.enable_text_time_segment_dotblink;
                da.queue_draw();
            }
        );
    }

    let menu_item_text_format_date = MenuItem::with_label( "Formant Date" );

    let menu_text_format_date = Menu::new();

    for x in AppInfoFormatDate::iter()
    {
        let menu_item = CheckMenuItem::with_label( x.format_str().1 );

        menu_item.set_active( app_info.borrow().text_format_date == x );

        {
            let da = da.clone();
            let app_info = app_info.clone();

            menu_item.connect_activate( move |_|
                {
                    let mut app_info = app_info.borrow_mut();
                    app_info.text_format_date = x;
                    da.queue_draw();
                }
            );
        }

        menu_text_format_date.append( &menu_item );
    }

    menu_item_text_format_date.set_submenu( Some( &menu_text_format_date ) );

    let menu_item_text_format_time = MenuItem::with_label( "Formant Time" );

    let menu_text_format_time = Menu::new();

    for x in AppInfoFormatTime::iter()
    {
        let menu_item = CheckMenuItem::with_label( x.format_str().1 );

        menu_item.set_active( app_info.borrow().text_format_time == x );

        {
            let da = da.clone();
            let app_info = app_info.clone();

            menu_item.connect_activate( move |_|
                {
                    let mut app_info = app_info.borrow_mut();
                    app_info.text_format_time = x;
                    da.queue_draw();
                }
            );
        }

        menu_text_format_time.append( &menu_item );
    }

    menu_item_text_format_time.set_submenu( Some( &menu_text_format_time ) );

    menu.append( &menu_item_enable_text_time_zone );
    menu.append( &menu_item_enable_text_date );
    menu.append( &menu_item_enable_text_time );
//    menu.append( &menu_item_enable_text_time_ampm );
    menu.append( &menu_item_enable_text_time_segment );
    menu.append( &menu_item_enable_text_time_segment_hour12 );
    menu.append( &SeparatorMenuItem::new() );
    menu.append( &menu_item_text_format_date );
    menu.append( &menu_item_text_format_time );

    menu
}

fn make_popup_menu(
    window: &ApplicationWindow,
    da: &DrawingArea,
    app_info: &Rc< RefCell< AppInfo > >,
    image_info: &Rc< RefCell< ImageInfo > >,
    logo: Option<Pixbuf>,
) -> Menu
{
    let with_second =
        if let Some( x ) = image_info.borrow().config.with_second && x { true } else { false }
        ;
    let with_sub_second =
        if let Some( x ) = image_info.borrow().config.with_sub_second && x { true } else { false }
        ;

    let menu = Menu::new();

    let menu_item_pref = MenuItem::with_label( "Preferences" );

    let menu_item_pref_alway_on_top = CheckMenuItem::with_label( "Alway on Top" );

    menu_item_pref_alway_on_top.set_active( app_info.borrow().always_on_top );

    {
        let window = window.clone();
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_pref_alway_on_top.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.always_on_top = !app_info.always_on_top;
                window.set_keep_above( app_info.always_on_top );
                da.queue_draw();
            }
        );
    }

    let menu_item_pref_lock_pos = CheckMenuItem::with_label( "Lock Position" );

    menu_item_pref_lock_pos.set_active( app_info.borrow().lock_pos );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_pref_lock_pos.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.lock_pos = !app_info.lock_pos;
                da.queue_draw();
            }
        );
    }

    let menu_item_pref_show_seconds = CheckMenuItem::with_label( "Show Seconds" );

    menu_item_pref_show_seconds.set_sensitive( with_second );
    menu_item_pref_show_seconds.set_active( app_info.borrow().show_seconds );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_pref_show_seconds.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.show_seconds = !app_info.show_seconds;
                da.queue_draw();
            }
        );
    }

    let menu_item_pref_enable_sub_second_hand = CheckMenuItem::with_label( "Enable sub second hand" );

    menu_item_pref_enable_sub_second_hand.set_sensitive( with_sub_second );
    menu_item_pref_enable_sub_second_hand.set_active( app_info.borrow().enable_sub_second_hand );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_pref_enable_sub_second_hand.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_sub_second_hand = !app_info.enable_sub_second_hand;
                da.queue_draw();
            }
        );
    }

    let menu_item_pref_enable_second_hand_smoothly  = CheckMenuItem::with_label( "Enable second hand smoothly" );

    menu_item_pref_enable_second_hand_smoothly .set_active( app_info.borrow().enable_second_hand_smoothly );

    {
        let da = da.clone();
        let app_info = app_info.clone();

        menu_item_pref_enable_second_hand_smoothly.connect_activate( move |_|
            {
                let mut app_info = app_info.borrow_mut();
                app_info.enable_second_hand_smoothly  = !app_info.enable_second_hand_smoothly ;
                da.queue_draw();
            }
        );
    }

    let menu_item_pref_text_visibility = MenuItem::with_label( "Text visibility" );

    let menu_pref_text_visibility = make_text_visibility_menu( &da.clone(), &app_info.clone(), &image_info.clone() );
    menu_item_pref_text_visibility.set_submenu( Some( &menu_pref_text_visibility ) );

    let menu_item_pref_time_zone = MenuItem::with_label( "Time Zone" );
    let menu_item_pref_theme = MenuItem::with_label( "Theme" );
    let menu_item_pref_zoom = MenuItem::with_label( "Zoom" );

    let menu_pref = Menu::new();

    menu_pref.append( &menu_item_pref_alway_on_top );
    menu_pref.append( &menu_item_pref_lock_pos );
    menu_pref.append( &SeparatorMenuItem::new() );
    menu_pref.append( &menu_item_pref_show_seconds );
    menu_pref.append( &menu_item_pref_enable_sub_second_hand );
    menu_pref.append( &menu_item_pref_enable_second_hand_smoothly );
    menu_pref.append( &SeparatorMenuItem::new() );
    menu_pref.append( &menu_item_pref_text_visibility );
    menu_pref.append( &SeparatorMenuItem::new() );
    menu_pref.append( &menu_item_pref_time_zone );
    menu_pref.append( &menu_item_pref_theme );
    menu_pref.append( &menu_item_pref_zoom );

    menu_item_pref.set_submenu( Some( &menu_pref ) );

    let menu_pref_time_zone = make_timezone_menu( &da.clone(), &app_info.clone() );
    menu_item_pref_time_zone.set_submenu( Some( &menu_pref_time_zone ) );

    let menu_pref_theme = make_theme_menu( &image_info.clone(), &app_info.clone() );
    menu_item_pref_theme.set_submenu( Some( &menu_pref_theme ) );

    let menu_pref_zoom = make_zoom_menu( &app_info.clone() );
    menu_item_pref_zoom.set_submenu( Some( &menu_pref_zoom ) );

    let menu_item_about = MenuItem::with_label( "About" );

    {
        let window = window.clone();

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
                about_dialog.set_parent( &window );
                about_dialog.show_all();
            }
        );
    }

    let menu_item_quit = MenuItem::with_label( "Quit" );

    {
        let window = window.clone();

        menu_item_quit.connect_activate(
            move |_| {
                window.close();
            }
        );
    }

    menu.append( &menu_item_pref );
    menu.append( &SeparatorMenuItem::new() );

    if app_info.borrow().time_disp_force.is_some()
    {
        let menu_item_snapshot = MenuItem::with_label( "Snapshot" );

        {
            let app_info = app_info.clone();
            let image_info = image_info.clone();

            menu_item_snapshot.connect_activate(
                move |_| {
                    make_png_image( &image_info.borrow(), &mut app_info.borrow_mut() );
                }
            );
        }

        menu.append( &menu_item_snapshot );
        menu.append( &SeparatorMenuItem::new() );
    }

    menu.append( &menu_item_about );
    menu.append( &menu_item_quit );

    menu
}

const UPDATE_CYCLE_SLOW: u64 = 100;
const UPDATE_CYCLE_FAST: u64 = 25;

fn get_timer_interval( is_fast: bool ) -> u64
{
    if is_fast { UPDATE_CYCLE_FAST } else { UPDATE_CYCLE_SLOW }
}
fn main() {

    pretty_env_logger::init();

    let app_info_file = ".hello_gtk";

    let app = Application::builder()
        .application_id("net.zuntan.example")
        .build();

    let app_info = Rc::new( RefCell::new( AppInfo::new() ) );

    {
        let file = File::open(app_info_file );

        match file
        {
            Ok( mut file ) =>
            {
                let mut buf = String::new();

                match file.read_to_string( &mut buf )
                {
                    Ok(_) =>
                    {
                        let app_info_load : Result< AppInfo, _> = toml::from_str( &buf );

                        match app_info_load
                        {
                            Ok( app_info_load ) =>
                            {
                                app_info.replace( app_info_load );
                            }
                        ,   Err( err ) => { error!( "{}", err ); }
                        }
                    }
                    ,   Err( err ) => { error!( "{}", err ); }
                }
            }
        ,   Err( err ) =>
            {
                match err.kind()
                {
                    std::io::ErrorKind::NotFound => { /* pass */ }
                ,   _ => { error!( "{}", err ); }
                }
            }
        }
    }

    // after setup
    app_info.borrow_mut().reset();

    {
        let app_info = app_info.clone();

        app.connect_activate(move |app| {

            let image_info = Rc::new( RefCell::new( load_theme( app_info.borrow().theme, app_info.borrow().theme_custom.clone()  ).unwrap() ) );

            let window = ApplicationWindow::builder()
                .application(app)
                .title("net.zuntan.example")
                .decorated(false)
                .tooltip_markup("example")
                .build();

            window.set_keep_above( app_info.borrow().always_on_top );

            if let Some( pos ) = app_info.borrow().window_pos
            {
                window.move_( pos.0, pos.1 );
            }

            let da =  DrawingArea::new();

            {
                let da = da.clone();
                let window = window.clone();
                let image_info = image_info.clone();
                let app_info = app_info.clone();

                da.connect_draw( move | da, cr |
                    {
                        update_watch( &da, &mut app_info.borrow_mut() );
                        update_region( &window, &image_info.borrow(), &mut app_info.borrow_mut() );
                        draw_watch(  cr, &image_info.borrow(), &app_info.borrow(), false );
                        gtk::glib::Propagation::Proceed
                    }
                );
            }

            window.add(&da);

            {
                let window = window.clone();
                let da = da.clone();
                let image_info = image_info.clone();
                let app_info = app_info.clone();

                window.connect_button_press_event( move | window,  evt |
                    {
                        // log::debug!("pressed: {:?}", evt.button() );

                        let logo = load_logo();

                        match evt.button()
                        {
                            1 => /* left button */
                            {
                                if !app_info.borrow().lock_pos
                                {
                                    let btn = evt.as_ref();
                                    window.begin_move_drag( btn.button as i32, btn.x_root as i32, btn.y_root as i32, btn.time );
                                }
                            }
                        ,   3 => /* right button */
                            {
                                let menu = make_popup_menu( &window, &da, &app_info, &image_info, logo );

                                menu.show_all();
                                menu.popup_at_pointer( Some( evt ) );

                                return gtk::glib::Propagation::Stop;
                            }
                        ,   _ => {}
                        }

                        gtk::glib::Propagation::Proceed
                    }
                );
            }

            {
                let app_info = app_info.clone();

                window.connect_delete_event(
                    move | window, _ |
                    {
                        log::debug!("connect_delete_event" );
                        log::debug!("pos:{:?}", window.position() );
                        app_info.borrow_mut().window_pos = Some( window.position() );
                        gtk::glib::Propagation::Proceed
                    }
                );
            }

            window.show_all();

            {
                let da = da.clone();
                let app_info = app_info.clone();

                app_info.borrow_mut().timer_sourceid.replace(
                    Some(
                        gtk::glib::source::timeout_add_local(
                            std::time::Duration::from_millis( get_timer_interval( false ) ),
                            move ||
                            {
                                da.queue_draw();
                                gtk::glib::ControlFlow::Continue
                            }
                        )
                    )
                );
            }
        });
    }

    {
        app.connect_shutdown(
            move | _ |
            {
                debug!("connect_shutdown");

                let toml_str = toml::to_string( app_info.as_ref() ).unwrap();
                let toml_str = format!( "#TOML\n\n{}", toml_str );

                let file = File::create(app_info_file );

                match file
                {
                    Ok( mut file ) =>
                    {
                        match file.write( toml_str.as_bytes() )
                        {
                            Ok(_) => {}
                        ,   Err(err) =>
                            {
                                error!( "{}", err );
                            }
                        }
                    },
                    Err( err ) =>
                    {
                        error!( "{}", err );
                    }
                }
            }
        );
    }

    app.run();

}
