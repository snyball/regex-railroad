use regex_syntax::Parser;
use regex_syntax::hir::*;
use std::env;
use std::mem;
#[macro_use]
extern crate lazy_static;

fn char_class(class: &ClassUnicode) -> Option<&'static str> {
    macro_rules! get_char_class {
        ($class:expr) => {
            match Parser::new().parse($class).unwrap().kind() {
                HirKind::Class(Class::Unicode(cls)) => cls.clone(),
                _ => panic!(),
            }
        };
    }

    macro_rules! test_class {
        ($class:expr, $hir:expr) => {{
            #![allow(non_upper_case_globals)]
            lazy_static! {
                static ref CLASS: ClassUnicode = get_char_class!($class);
            }
            let mut hir_c = $hir.clone();
            hir_c.symmetric_difference(&CLASS);
            if hir_c.iter().collect::<Vec<_>>().len() == 0 {
                return Some($class)
            }
        }};
    }

    test_class!(r"\d", class);
    test_class!(r"\w", class);
    test_class!(r"\s", class);
    test_class!(r"\S", class);
    test_class!(r"\W", class);
    test_class!(r"\D", class);
    test_class!(r".", class);
    None
}

fn is_everything_except(class: &ClassUnicode) -> Option<String> {
    const MAX_LEN: usize = 48;
    let mut nc = class.clone();
    nc.negate();

    let mut s = String::from("[^");
    for range in nc.iter() {
        let (a, b) = (range.start() as u32, range.end() as u32);
        for c in a..=b {
            let c = std::char::from_u32(c).unwrap();
            s.push(c);
            if s.len() > MAX_LEN {
                return None;
            }
        }
    }

    s.push_str("]");

    Some(s)
}

fn py_char_str(c: char) -> String {
    if c == '\'' {
        r#"'\''"#.to_string()
    } else if c == '\\' {
        r#"'\\'"#.to_string()
    } else {
        format!(r#"'{}'"#, c)
    }
}

fn py_str(s: &str) -> String {
    let mut ns = String::new();
    ns.push('\'');
    for c in s.chars() {
        if c == '\'' {
            ns.push_str(r#"\'"#);
        } else if c.is_control() || c as u32 >= 0xffff {
            ns.push_str(&format!("<{:X}>", c as u32));
        } else {
            ns.push(c);
        }
    }
    ns.push('\'');
    ns
}

fn descent(root: &Hir) {
    match root.kind() {
        HirKind::Alternation(hirs) => {
            print!("Choice(0, ");
            for hir in hirs {
                descent(hir);
                print!(", ")
            }
            print!(")");
        }
        HirKind::Group(grp) => {
            print!("Group(");
            let name = match &grp.kind {
                GroupKind::CaptureName { name, .. } => name.clone(),
                GroupKind::CaptureIndex(idx) => idx.to_string(),
                GroupKind::NonCapturing => {
                    descent(&grp.hir);
                    print!(")");
                    return;
                }
            };
            descent(&grp.hir);
            print!(", {})", py_str(&name));
        }
        HirKind::Literal(Literal::Unicode(lit)) => print!("{}", py_char_str(*lit)),
        HirKind::Repetition(rep) => {
            print!("{}(", match &rep.kind {
                RepetitionKind::OneOrMore => "OneOrMore",
                RepetitionKind::ZeroOrMore => "ZeroOrMore",
                RepetitionKind::ZeroOrOne => "Optional",
                x => unimplemented!("{:?}", x),
            });
            descent(&rep.hir);
            print!(")");
        }
        HirKind::Concat(hirs) => {
            print!("Sequence(");
            for hir in hirs.iter() {
                descent(hir);
                print!(", ")
            }
            print!(")");
        }
        HirKind::Class(Class::Unicode(class)) => {
            if let Some(c) = char_class(class) {
                print!("{}", py_str(c));
            } else if let Some(c) = is_everything_except(class) {
                print!("{}", py_str(&c));
            } else {
                print!("Choice(0, ");
                for (i, range) in class.iter().enumerate() {
                    if i >= 20 {
                        print!("{:?}", "...");
                        break;
                    }
                    if range.start() == range.end() {
                        print!("{}, ", py_char_str(range.start()));
                    } else {
                        print!("{}, ", py_str(&format!("{}-{}",
                                                       range.start(),
                                                       range.end())));
                    }
                }
                print!(")");
            }
        }
        HirKind::Anchor(Anchor::StartLine) => print!("{:?}", "^"),
        HirKind::Anchor(Anchor::EndLine) => print!("{:?}", "$"),
        HirKind::Anchor(Anchor::StartText) => print!("{:?}", "^"),
        HirKind::Anchor(Anchor::EndText) => print!("{:?}", "$"),
        HirKind::WordBoundary(_) => print!("{}", py_str(r"\b")),
        HirKind::Empty => print!("{}", py_str("")),
        x => unimplemented!("{:?}", x)
    }
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 1 {
        panic!("Invalid number of arguments");
    }
    let rx = &args[0];
    let hir = Parser::new().parse(rx).unwrap();
    println!("import sys");
    println!("from railroad import *");
    print!("Diagram({:?}, ", "start");
    descent(&hir);
    println!(").writeSvg(sys.stdout.write)");
}
