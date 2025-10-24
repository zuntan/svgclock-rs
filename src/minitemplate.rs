use regex::Regex;
use std::fmt;
use std::any::Any;
use std::collections::{HashSet, HashMap};
use std::io::Cursor;
use std::io::prelude::*;
use std::io::{self, BufRead, BufReader, Read};
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::sync::LazyLock;

/*
pub fn any_to_str(item: &dyn Any) -> String {
    if let Some(s) = item.downcast_ref::<Rc<dyn Any>>() {
        any_to_str(&**s)
    } else if let Some(s) = item.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = item.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(b) = item.downcast_ref::<bool>() {
        b.to_string()
    } else if let Some(num) = item.downcast_ref::<i32>() {
        num.to_string()
    } else if let Some(num) = item.downcast_ref::<f64>() {
        num.to_string()
    } else if let Some(s) = item.downcast_ref::<Box<dyn Any>>() {
        any_to_str(&**s)
    } else {
        String::new()
    }
}
*/

pub fn escape_str(item: String, opt: &VarOpt) -> String {
    static R: LazyLock<Regex> = LazyLock::new(|| Regex::new("[<>&'\" ]").unwrap());

    match opt {
        VarOpt::HTML | VarOpt::XML => {
            let escaped = R.replace_all(&item, |caps: &regex::Captures| {
                match caps.get(0).unwrap().as_str() {
                    "&" => "&amp;",
                    "<" => "&lt;",
                    ">" => "&gt;",
                    "\"" => "&quot;",
                    "'" => "&#39;",
                    " " => "&nbsp;",
                    _ => "",
                }
            });

            /*
            item
                .replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("\"", "&quot;")
                .replace("'", "&#39;")
            */

            escaped.into_owned()
        }
        _ => item,
    }
}

pub type VecValue = Vec< Rc< ContextValue > >;
pub type MapValue = HashMap< String, Rc< ContextValue > >;

#[derive(Clone)]
pub enum ContextValue {
    String( String ),
    Bool( bool ),
    I32( i32 ),
    I64( i64 ),
    F64( f64 ),
    MapValue( MapValue ),
    VecValue( VecValue )
}

impl From< String > for ContextValue {
    fn from(value: String) -> Self {
        ContextValue::String( value )
    }
}

impl From< &str > for ContextValue {
    fn from(value: &str) -> Self {
        ContextValue::String( value.to_string() )
    }
}

impl From< i32 > for ContextValue {
    fn from(value: i32) -> Self {
        ContextValue::I32( value )
    }
}

impl From< i64 > for ContextValue {
    fn from(value: i64) -> Self {
        ContextValue::I64( value )
    }
}

impl From< f64 > for ContextValue {
    fn from(value: f64) -> Self {
        ContextValue::F64( value )
    }
}

impl From< MapValue > for ContextValue {
    fn from(value: MapValue) -> Self {
        ContextValue::MapValue( value )
    }
}

impl From< VecValue > for ContextValue {
    fn from(value: VecValue) -> Self {
        ContextValue::VecValue( value )
    }
}

pub trait ContextTrait {

    fn put_item(&mut self, key: &str, val: ContextValue ) -> Option< Rc< ContextValue > >;

    fn get_item(&self, key: &str) -> Option< Rc< ContextValue > >;
    fn get_bool(&self, key: &str) -> bool;
    fn get_str(&self, key: &str, opt: &VarOpt) -> String;
    fn get_vec(&self, key: &str) -> Option< Vec< Rc< ContextValue > > >;

    fn push(&mut self, name: &str, item: &Rc< ContextValue > );
    fn pop(&mut self);
}

pub trait WriteTo {
    fn write_to<W: Write>(&self, ctx: &mut dyn ContextTrait, out: &mut W) -> io::Result<()>;
}

#[derive(Debug, Clone)]
struct Block {
    parts: Vec<Part>,
}

impl Block {
    fn new() -> Block {
        Block { parts: Vec::new() }
    }
}

#[derive(Debug, Clone)]
struct Str {
    str: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarOpt {
    DEF,
    RAW,
    HTML,
    XML,
}

#[derive(Debug, Clone)]
struct Var {
    key: String,
    opt: VarOpt,
}

#[derive(Debug, Clone)]
struct CondBlock {
    key: String,
    pos: Block,
    neg: Block,
}

#[derive(Debug, Clone)]
struct LoopBlock {
    name: String,
    key: String,
    inner: Block,
}

#[derive(Debug, Clone)]
enum Part {
    Str(Str),
    Var(Var),
    CondBlock(CondBlock),
    LoopBlock(LoopBlock),
}

impl WriteTo for Block {
    fn write_to<W: Write>(&self, ctx: &mut dyn ContextTrait, out: &mut W) -> io::Result<()> {
        for part in self.parts.iter() {
            part.write_to(ctx, out)?;
        }
        Ok(())
    }
}
impl WriteTo for Part {
    fn write_to<W: Write>(&self, ctx: &mut dyn ContextTrait, out: &mut W) -> io::Result<()> {
        match self {
            Part::Str(part) => {
                out.write(part.str.as_bytes())?;
            }
            Part::Var(part) => {
                out.write(ctx.get_str(&part.key, &part.opt).as_bytes())?;
            }
            Part::CondBlock(part) => {
                if ctx.get_bool(&part.key) {
                    &part.pos
                } else {
                    &part.neg
                }
                .write_to(ctx, out)?;
            }
            Part::LoopBlock(part) => {
                let item = ctx.get_vec(&part.key);

                if let Some(vec) = item {
                    for x in vec {
                        ctx.push(&part.name, &x );
                        part.inner.write_to(ctx, out)?;
                        ctx.pop();
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Template {
    block: Block,
}

impl Template {
    fn new() -> Template {
        Template {
            block: Block::new(),
        }
    }

    pub fn write_to<W: Write>(&self, ctx: &mut dyn ContextTrait, out: &mut W) -> io::Result<()> {
        self.block.write_to(ctx, out)?;
        Ok(())
    }

    pub fn render(&self, ctx: &mut dyn ContextTrait) -> String {
        let mut cursor = Cursor::new(Vec::new());
        self.write_to(ctx, &mut cursor);
        String::from_utf8(cursor.into_inner()).unwrap()
    }

    pub fn get_varnames(&self) -> HashSet< String > {

        fn get_varnames_from_block( v: &mut HashSet< String >, b : &Block ) {

            for part in b.parts.iter() {
                match part {
                    Part::Str(part) => {
                    }
                    Part::Var(part) => {
                        v.insert( part.key.to_string() );
                    }
                    Part::CondBlock(part) => {
                        v.insert( part.key.to_string() );
                        get_varnames_from_block(v, &part.pos);
                        get_varnames_from_block(v, &part.neg);
                    }
                    Part::LoopBlock(part) => {
                        v.insert( part.key.to_string() );
                        get_varnames_from_block(v, &part.inner);
                    }
                }
            }
        }

        let mut v: HashSet< String > = HashSet::new();

        get_varnames_from_block( &mut v, &self.block );

        v
    }
}

#[derive(Debug)] // デバッグ出力のために必須
pub enum ParserError {
    Io(io::Error),
    InvalidData(String),
    ParserError((usize, usize, String)),
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParserError::Io(err) => {
                write!(f, "I/O Error: {}", err)
            }
            ParserError::InvalidData(msg) => {
                write!(f, "Error : {}", msg)
            }
            ParserError::ParserError(param) => {

                if param.0 == usize::MAX {
                    write!(f, "Parse error at end of text. {}", param.2 )
                }
                else {
                    write!(f, "Parse error at Line:{} Col:{}. {}", param.0,  param.1,  param.2 )
                }
            }
        }
    }
}

pub fn parse<R: Read>(source: R) -> Result<Template, ParserError> {
    enum StackBlockType {
        Block,
        CondPos,
        CondNeg,
        LoopInner,
    }

    enum StackPart {
        Comment( usize ),
        Block( ( usize, Block, StackBlockType, String, String ) ),
    }

    static TOKEN_NONE: &str = "";
    static TOKEN_VAR_S: &str = "{{";
    static TOKEN_VAR_E: &str = "}}";
    static TOKEN_ST_S: &str = "{%";
    static TOKEN_ST_E: &str = "%}";
    static TOKEN_C_S: &str = "{#";
    static TOKEN_C_E: &str = "#}";

    static R_TOKEN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new("(\\{\\{|\\}\\}|\\{%|%\\}|\\{#|#\\})").unwrap());
    static R_VAR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new("\\s*([A-Za-z0-9-._]+)(?:\\s*:\\s*(r|raw|html|xml))?\\s*").unwrap()
    });
    static R_ST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            "\\s*(if|else|for|end)(?:\\s+([A-Za-z0-9-._]+)(?:\\s+in\\s+([A-Za-z0-9-._]+))?)?\\s*",
        )
        .unwrap()
    });

    let mut part_stack: Vec<StackPart> = Vec::new();

    part_stack.push(StackPart::Block((
        0,
        Block::new(),
        StackBlockType::Block,
        String::new(),
        String::new(),
    )));

    let mut pa_line: usize = 0;
    let mut pa_col: usize = 0;

    let mut line = String::new();

    let mut buf: String = String::new();
    let mut buf_token: String = String::new();
    let mut buf_token_line: usize = 0;
    let mut buf_token_col: usize = 0;

    let mut reader = BufReader::new(source);

    // for x in reader.lines() {
    loop {
        line.clear();
        let r = reader.read_line(&mut line);

        match r {
            Ok(len) => {
                if len == 0 {
                    break;
                }

                pa_line = pa_line + 1;
                pa_col = 0;

                for caps in R_TOKEN.captures_iter(&line) {
                    let top = part_stack.last_mut().unwrap();

                    let m0 = caps.get(0).unwrap();

                    buf.push_str(&line[pa_col..m0.start()]);
                    pa_col = m0.start();

                    let token = m0.as_str();

                    match top {
                        StackPart::Comment(_) => {
                            if token == TOKEN_C_E {
                                if part_stack.pop().is_none() {
                                    return Err(ParserError::ParserError((
                                        pa_line,
                                        pa_col,
                                        String::from( "NoSuchElement" ),
                                    )));
                                }

                                buf.clear();

                            } else if token == TOKEN_C_S {
                                part_stack.push(StackPart::Comment(pa_line));
                            }
                        }
                        StackPart::Block(block_pair) => {
                            if token == TOKEN_C_E {
                                return Err(ParserError::ParserError((
                                    pa_line,
                                    pa_col,
                                    format!( "`{}`", token ),
                                )));
                            } else if token == TOKEN_C_S {

                                block_pair.1.parts.push(Part::Str(Str { str: buf.clone() }));
                                buf.clear();

                                part_stack.push(StackPart::Comment(pa_line));

                            } else if token == TOKEN_ST_E {
                                if buf_token != TOKEN_ST_S {
                                    return Err(ParserError::ParserError((
                                        pa_line,
                                        pa_col,
                                        format!( "`{}`", token ),
                                    )));
                                }

                                if buf_token == TOKEN_NONE {
                                    return Err(ParserError::ParserError((
                                        pa_line,
                                        pa_col,
                                        format!( "`{}`", token ),
                                    )));
                                }

                                match R_ST.captures(&buf) {
                                    Some(cap) => {
                                        let op = cap.get(1).unwrap().as_str();

                                        match op {
                                            "end" => match block_pair.2 {
                                                StackBlockType::CondPos => {
                                                    let block1 = block_pair.1.clone();
                                                    let param1 = block_pair.3.clone();

                                                    part_stack.pop();

                                                    let top_new = part_stack.last_mut().unwrap();

                                                    if let StackPart::Block(block_pair_new) =
                                                        top_new
                                                    {
                                                        block_pair_new.1.parts.push(
                                                            Part::CondBlock(CondBlock {
                                                                key: param1,
                                                                pos: block1,
                                                                neg: Block::new(),
                                                            }),
                                                        )
                                                    }
                                                }
                                                StackBlockType::CondNeg => {
                                                    let block2 = block_pair.1.clone();

                                                    part_stack.pop();

                                                    let cond_pos = part_stack.pop().unwrap();

                                                    if let StackPart::Block(block_pair1) = cond_pos
                                                    {
                                                        match block_pair1.2 {
                                                            StackBlockType::CondPos => {
                                                                let block1 = block_pair1.1.clone();
                                                                let param1 = block_pair1.3.clone();

                                                                let top_new =
                                                                    part_stack.last_mut().unwrap();

                                                                if let StackPart::Block(
                                                                    block_pair_new,
                                                                ) = top_new
                                                                {
                                                                    block_pair_new.1.parts.push(
                                                                        Part::CondBlock(
                                                                            CondBlock {
                                                                                key: param1,
                                                                                pos: block1,
                                                                                neg: block2,
                                                                            },
                                                                        ),
                                                                    )
                                                                }
                                                            }
                                                            _ => {
                                                                return Err(
                                                                    ParserError::ParserError((
                                                                        buf_token_line,
                                                                        buf_token_col,
                                                                        String::from( "NoSuchElement" ),
                                                                    )),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                                StackBlockType::LoopInner => {
                                                    let block1 = block_pair.1.clone();
                                                    let param1 = block_pair.3.clone();
                                                    let param2 = block_pair.4.clone();

                                                    part_stack.pop();

                                                    let top_new = part_stack.last_mut().unwrap();

                                                    if let StackPart::Block(block_pair_new) =
                                                        top_new
                                                    {
                                                        block_pair_new.1.parts.push(
                                                            Part::LoopBlock(LoopBlock {
                                                                name: param1,
                                                                key: param2,
                                                                inner: block1,
                                                            }),
                                                        )
                                                    }
                                                }
                                                _ => {
                                                    return Err(ParserError::ParserError((
                                                        buf_token_line,
                                                        buf_token_col,
                                                        String::from( "NoSuchElement" ),
                                                    )));
                                                }
                                            },
                                            "else" => match block_pair.2 {
                                                StackBlockType::CondPos => {
                                                    part_stack.push(StackPart::Block((
                                                        buf_token_line,
                                                        Block::new(),
                                                        StackBlockType::CondNeg,
                                                        String::new(),
                                                        String::new(),
                                                    )));
                                                }
                                                _ => {
                                                    return Err(ParserError::ParserError((
                                                        buf_token_line,
                                                        buf_token_col,
                                                        format!( "`{}`", token ),
                                                    )));
                                                }
                                            },
                                            "if" => {
                                                let key = cap.get(2).unwrap().as_str();
                                                part_stack.push(StackPart::Block((
                                                    buf_token_line,
                                                    Block::new(),
                                                    StackBlockType::CondPos,
                                                    String::from(key),
                                                    String::new(),
                                                )));
                                            }
                                            "for" => {
                                                let name = cap.get(2).unwrap().as_str();

                                                if let Some(cap2) = cap.get(3) {
                                                    let iter = cap2.as_str();
                                                    part_stack.push(StackPart::Block((
                                                        buf_token_line,
                                                        Block::new(),
                                                        StackBlockType::LoopInner,
                                                        String::from(name),
                                                        String::from(iter),
                                                    )));
                                                } else {
                                                    return Err(ParserError::ParserError((
                                                        buf_token_line,
                                                        buf_token_col,
                                                        format!( "`{}`", token ),
                                                    )));
                                                }
                                            }
                                            _ => {
                                                return Err(ParserError::ParserError((
                                                    buf_token_line,
                                                    buf_token_col,
                                                    format!( "`{}`", token ),
                                                )));
                                            }
                                        }
                                    }
                                    None => {
                                        return Err(ParserError::ParserError((
                                            buf_token_line,
                                            buf_token_col,
                                            format!( "`{}`", token ),
                                        )));
                                    }
                                }

                                buf.clear();
                                buf_token = String::new();
                                buf_token_line = 0;
                                buf_token_col = 0;
                            } else if token == TOKEN_VAR_E {
                                if buf_token != TOKEN_VAR_S {
                                    return Err(ParserError::ParserError((
                                        pa_line,
                                        pa_col,
                                        format!( "`{}`", token ),
                                    )));
                                }

                                if buf_token == TOKEN_NONE {
                                    return Err(ParserError::ParserError((
                                        pa_line,
                                        pa_col,
                                        format!( "`{}`", token ),
                                    )));
                                }

                                match R_VAR.captures(&buf) {
                                    Some(cap) => {
                                        let key = cap.get(1).unwrap().as_str();
                                        let opt = if let Some(x) = cap.get(2) {
                                            match x.as_str() {
                                                "r" => VarOpt::RAW,
                                                "raw" => VarOpt::RAW,
                                                "html" => VarOpt::HTML,
                                                "xml" => VarOpt::XML,
                                                _ => VarOpt::DEF,
                                            }
                                        } else {
                                            VarOpt::DEF
                                        };

                                        block_pair.1.parts.push(Part::Var(Var {
                                            key: key.to_string(),
                                            opt: opt,
                                        }));
                                    }
                                    None => {
                                        return Err(ParserError::ParserError((
                                            buf_token_line,
                                            buf_token_col,
                                            format!( "`{}`", token ),
                                        )));
                                    }
                                }

                                buf.clear();
                                buf_token = String::new();
                                buf_token_line = 0;
                                buf_token_col = 0;
                            } else if token == TOKEN_ST_S || token == TOKEN_VAR_S {
                                if buf_token != TOKEN_NONE {
                                    return Err(ParserError::ParserError((
                                        pa_line,
                                        pa_col,
                                        format!( "`{}`", token ),
                                    )));
                                }

                                block_pair.1.parts.push(Part::Str(Str { str: buf.clone() }));

                                buf.clear();
                                buf_token = String::from(token);
                                buf_token_line = pa_line;
                                buf_token_col = pa_col;
                            }
                        }
                    }

                    pa_col = m0.end();
                }

                buf.push_str(&line[pa_col..])
            }

            Err(e) => {
                return Err(ParserError::Io(e));
            }
        }
    }

    if buf_token != TOKEN_NONE {
        let token_pair = if buf_token == TOKEN_VAR_S {
            TOKEN_VAR_E
        } else if buf_token == TOKEN_ST_S {
            TOKEN_ST_E
        } else {
            TOKEN_NONE
        };

        return Err(ParserError::ParserError((
            usize::MAX,
            0,
            format!("Missing `{}` /From Line:{} Col:{} `{}`", token_pair, buf_token_line, buf_token_col, buf_token )
        )));
    }

    if part_stack.len() != 1 {
        let top = part_stack.last().unwrap();

        let missing = match top {
            StackPart::Comment(line) => ("#}", TOKEN_C_S, *line ),
            StackPart::Block(brock_part) => match brock_part.2 {
                StackBlockType::CondNeg | StackBlockType::CondPos => ("{% end %}", "{% if", brock_part.0 ),
                StackBlockType::LoopInner => ("{% end %}", "{% for", brock_part.0 ),
                _ => ( "", "", brock_part.0 ),
            },
        };
        return Err(ParserError::ParserError((
            usize::MAX,
            0,
           format!( "Missing `{}` /From Line:{} `{}`", missing.0, missing.2, missing.1 )
        )));
    }

    let mut t = Template::new();

    let top = part_stack.last_mut().unwrap();

    match top {
        StackPart::Block(block_pair) => {
            block_pair.1.parts.push(Part::Str(Str { str: buf.clone() }));
            t.block = block_pair.1.clone();
        }
        _ => {
            return Err(ParserError::ParserError((
                usize::MAX,
                0,
                String::new()
            )));
        }
    }

    Ok(t)
}

pub fn parse_str(source: &str) -> Result<Template, ParserError> {
    parse(source.as_bytes())
}

pub struct Context {
    pub opt: VarOpt,
    pub is_def_blank: bool,
    dict: HashMap<String, Rc< ContextValue > >,
    stack: Vec<(String, Weak< ContextValue > ) >,
}

impl Context {
    pub fn new() -> Context {
        Context {
            stack: Vec::new(),
            opt: VarOpt::HTML,
            is_def_blank: false,
            dict: HashMap::new(),
        }
    }
}

impl ContextTrait for Context {

    fn put_item(&mut self, key: &str, val: ContextValue ) -> Option< Rc< ContextValue > > {
        self.dict.insert( key.to_string(), Rc::new( val ) )
    }

    fn get_item(&self, key: &str) -> Option< Rc< ContextValue > > {
        let keys = key.split_once('.').unwrap_or( ( key, "" ) );

        for x in self.stack.iter().rev() {
            if x.0 == keys.0 {
                if let Some(item) = x.1.upgrade() {

                    match item.as_ref() {
                        ContextValue::MapValue( dict) => {
                            if let Some( item ) = dict.get(keys.1) {
                                return Some( item.clone() );
                            }
                            return None;
                        }
                        _ => {
                            return Some( item );
                        }
                    }
                }
            }
        }

        if let Some( item ) = self.dict.get(key) {
            Some( item.clone() )
        }
        else {
            None
        }
    }

    fn get_bool(&self, key: &str) -> bool {
        let item = self.get_item(key);

        if let Some(item) = item {

            match item.as_ref() {
                ContextValue::Bool( b) => {
                    return *b;
                }
                ContextValue::String( s) => {
                    let normalized = s.trim().to_lowercase();

                    return match normalized.as_str() {
                        "true" | "t" | "1" | "yes" | "y" => true,
                        _ => false,
                    };
                }
                ContextValue::I32( v)   => {
                    return *v != 0;
                }
                ContextValue::I64( v)   => {
                    return *v != 0;
                }
                ContextValue::F64( v)   => {
                    return *v != 0.0;
                }
                _ => {
                }
            }
        }

        false
    }

    fn get_str(&self, key: &str, opt: &VarOpt) -> String {
        let item = self.get_item(key);

        let ret = if let Some(item) = item {

                match item.as_ref() {
                    ContextValue::Bool( b) => {
                        b.to_string()
                    }
                    ContextValue::String( s) => {
                        s.to_string()
                    }
                    ContextValue::I32( v)   => {
                        v.to_string()
                    }
                    ContextValue::I64( v)   => {
                        v.to_string()
                    }
                    ContextValue::F64( v)   => {
                        v.to_string()
                    }
                    _ => {
                        String::new()
                    }
                }
            }
            else {

                if self.is_def_blank {
                    String::new()
                }
                else {
                    format!( "???{}???", key )
                }
            }
            ;

        let opt = if *opt == VarOpt::DEF { &self.opt } else { opt };

        escape_str(ret, opt)
    }

    fn get_vec(&self, key: &str) -> Option< Vec< Rc< ContextValue > > > {
        let item = self.get_item(key);

        if let Some(item) = item {
            if let ContextValue::VecValue(vec ) = item.as_ref() {
                return Some( vec.clone() );
            }
        }

        None
    }

    fn push(&mut self, name: &str, item: &Rc< ContextValue > ) {
        self.stack.push( ( name.to_string(), Rc::downgrade( item) ) );
    }

    fn pop(&mut self) {
        self.stack.pop();
    }
}

impl Deref for Context {
    type Target = HashMap<String, Rc< ContextValue > >;

    fn deref(&self) -> &Self::Target {
        &self.dict
    }
}

impl DerefMut for Context {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.dict
    }
}

pub trait SetValue<T> {
    fn set(&mut self, k: &str, v: T) -> Option< Rc< ContextValue > >;
}

impl SetValue<String> for Context {
    fn set(&mut self, k: &str, v: String) -> Option< Rc< ContextValue > > {
        self.dict.insert(k.to_string(), Rc::new( ContextValue::String( v ) ) )
    }
}

impl SetValue<&str> for Context {
    fn set(&mut self, k: &str, v: &str) -> Option< Rc< ContextValue > > {
        self.dict.insert(k.to_string(), Rc::new( ContextValue::String( v.to_string() ) ) )
    }
}

impl SetValue<bool> for Context {
    fn set(&mut self, k: &str, v: bool) -> Option< Rc< ContextValue > > {
        self.dict.insert(k.to_string(), Rc::new( ContextValue::Bool( v ) ) )
    }
}

impl SetValue<i32> for Context {
    fn set(&mut self, k: &str, v: i32) -> Option< Rc< ContextValue > > {
        self.dict.insert(k.to_string(), Rc::new( ContextValue::I32( v ) ) )
    }
}

impl SetValue<i64> for Context {
    fn set(&mut self, k: &str, v: i64) -> Option< Rc< ContextValue > > {
        self.dict.insert(k.to_string(), Rc::new( ContextValue::I64( v ) ) )
    }
}

impl SetValue<f64> for Context {
    fn set(&mut self, k: &str, v: f64) -> Option< Rc< ContextValue > > {
        self.dict.insert(k.to_string(), Rc::new( ContextValue::F64( v ) ) )
    }
}

impl SetValue<VecValue> for Context {
    fn set(&mut self, k: &str, v: VecValue) -> Option< Rc< ContextValue > > {
        self.dict.insert(k.to_string(), Rc::new( ContextValue::VecValue( v ) ) )
    }
}

pub trait SetValueForVecValue<T> {
    fn set(&mut self, v: T);
}

impl SetValueForVecValue<String> for VecValue {
    fn set(&mut self, v: String) {
        self.push( Rc::new( ContextValue::String( v ) ) );
    }
}

impl SetValueForVecValue<&str> for VecValue {
    fn set(&mut self, v: &str) {
        self.push( Rc::new( ContextValue::String( v.to_string() ) ) );
    }
}

impl SetValueForVecValue<bool> for VecValue {
    fn set(&mut self, v: bool) {
        self.push( Rc::new( ContextValue::Bool( v ) ) );
    }
}

impl SetValueForVecValue<i32> for VecValue {
    fn set(&mut self, v: i32) {
        self.push( Rc::new( ContextValue::I32( v ) ) );
    }
}

impl SetValueForVecValue<i64> for VecValue {
    fn set(&mut self, v: i64) {
        self.push( Rc::new( ContextValue::I64( v ) ) );
    }
}

impl SetValueForVecValue<f64> for VecValue {
    fn set(&mut self, v: f64) {
        self.push( Rc::new( ContextValue::F64( v ) ) );
    }
}

impl SetValueForVecValue<VecValue> for VecValue {
    fn set(&mut self, v: VecValue) {
        self.push( Rc::new( ContextValue::VecValue( v ) ) );
    }
}

impl SetValueForVecValue<MapValue> for VecValue {
    fn set(&mut self, v: MapValue) {
        self.push( Rc::new( ContextValue::MapValue( v ) ) );
    }
}

impl SetValue<String> for MapValue {
    fn set(&mut self, k: &str, v: String) -> Option< Rc< ContextValue > > {
        self.insert(k.to_string(), Rc::new( ContextValue::String( v ) ) )
    }
}

impl SetValue<&str> for MapValue {
    fn set(&mut self, k: &str, v: &str) -> Option< Rc< ContextValue > > {
        self.insert(k.to_string(), Rc::new( ContextValue::String( v.to_string() ) ) )
    }
}

impl SetValue<bool> for MapValue {
    fn set(&mut self, k: &str, v: bool) -> Option< Rc< ContextValue > > {
        self.insert(k.to_string(), Rc::new( ContextValue::Bool( v ) ) )
    }
}

impl SetValue<i32> for MapValue {
    fn set(&mut self, k: &str, v: i32) -> Option< Rc< ContextValue > > {
        self.insert(k.to_string(), Rc::new( ContextValue::I32( v ) ) )
    }
}

impl SetValue<i64> for MapValue {
    fn set(&mut self, k: &str, v: i64) -> Option< Rc< ContextValue > > {
        self.insert(k.to_string(), Rc::new( ContextValue::I64( v ) ) )
    }
}

impl SetValue<f64> for MapValue {
    fn set(&mut self, k: &str, v: f64) -> Option< Rc< ContextValue > > {
        self.insert(k.to_string(), Rc::new( ContextValue::F64( v ) ) )
    }
}

impl SetValue<VecValue> for MapValue {
    fn set(&mut self, k: &str, v: VecValue) -> Option< Rc< ContextValue > > {
        self.insert(k.to_string(), Rc::new( ContextValue::VecValue( v ) ) )
    }
}

pub fn make_vec_value( slice: &[ ContextValue ] ) -> VecValue {
    slice
        .iter()
        .map( | x | {
            Rc::new( x.clone() )
        } )
        .collect()
}

pub fn make_map_value( slice: &[ (&str, ContextValue ) ] ) -> MapValue {
    slice
        .iter()
        .map( | x | {
            ( x.0.to_string(), Rc::new( x.1.clone() ) )
        } )
        .collect()
}

pub fn trim_margin(text: &str, margin_prefix: Option< &str >) -> String {
    let margin_prefix= margin_prefix.unwrap_or( "|" );

    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with(margin_prefix) {
                &trimmed[margin_prefix.len()..]
            } else {
                trimmed
            }
        })
        .collect::<Vec<&str>>()
        .join("\n")
        .trim_start()
        .to_string()
}
