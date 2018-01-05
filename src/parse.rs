use std;
use std::str::FromStr;
use unicode_xid::UnicodeXID;

use context::{Context, NodeInfo};
use diagram::{Diagram, MatchTerm, MatchTermConstraint, Node, OutputTerm};
use fixgraph::NodeIndex;
use graph_diagram::GraphDiagram;
use predicate::Predicate;
use value::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error<'a> {
    Msg { msg: &'static str, rest: &'a str },
}

type Result<'a, T> = std::result::Result<(T, &'a str), Error<'a>>;

type EmptyResult<'a> = std::result::Result<&'a str, Error<'a>>;

fn err_msg<'a, T>(msg: &'static str, rest: &'a str) -> Result<'a, T> {
    Err(err_from_str(msg, rest))
}

fn err_from_str<'a>(msg: &'static str, rest: &'a str) -> Error<'a> {
    Error::Msg {
        msg: msg,
        rest: rest,
    }
}

fn some_char_is<F>(opt_char: Option<char>, f: F) -> bool
where
    F: Fn(char) -> bool,
{
    if let Some(c) = opt_char {
        f(c)
    } else {
        false
    }
}

fn substr_index(src: &str, substr: &str) -> usize {
    let diff = substr.as_ptr() as isize - src.as_ptr() as isize;
    if 0 <= diff && diff as usize <= src.len() {
        diff as usize
    } else {
        panic!("substr_index called with invalid substr")
    }
}

fn slice_src<'a>(src: &'a str, rest: &'a str) -> &'a str {
    let index = substr_index(src, rest);
    src.split_at(index).0
}

fn character_is<F>(src: &str, f: F) -> Result<char>
where
    F: Fn(char) -> bool,
{
    let mut cs = src.chars();
    let c = cs.next();
    if some_char_is(c, f) {
        let rest = cs.as_str();
        return Ok((c.unwrap(), rest));
    } else {
        return err_msg("Wrong character", src);
    }
}

fn character(src: &str, c: char) -> Result<char> {
    character_is(src, |x| c == x)
}

fn start_and_continue<F, G>(src: &str, f: F, g: G) -> Result<&str>
where
    F: Fn(char) -> bool,
    G: Fn(char) -> bool,
{
    let mut rest;
    let mut cs = src.chars();
    if some_char_is(cs.next(), f) {
        rest = cs.as_str();
    } else {
        return err_msg("Wrong starting character", src);
    }
    while some_char_is(cs.next(), &g) {
        rest = cs.as_str();
    }
    return Ok((slice_src(src, rest), rest));
}

fn prefix<'a, 'b>(src: &'a str, prefix: &'b str) -> EmptyResult<'a> {
    let mut rest = src;
    let mut cs = src.chars();
    let mut ps = prefix.chars();
    while let Some(p) = ps.next() {
        if let Some(c) = cs.next() {
            if p == c {
                rest = cs.as_str();
                continue;
            }
        }
        return Err(Error::Msg {
            msg: "Prefix did not match",
            rest,
        });
    }
    return Ok(rest);
}

fn unsigned_decimal_integer(src: &str) -> Result<u64> {
    if let Ok((_, rest)) = character(src, '0') {
        if character_is(rest, |c| c.is_digit(10)).is_ok() {
            err_msg("Octal literal", src)
        } else {
            Ok((0, rest))
        }
    } else {
        let (num_src, rest) = start_and_continue(src, |c| c.is_digit(10), |c| c.is_digit(10))?;
        Ok((u64::from_str(num_src).unwrap(), rest))
    }
}

fn char_is_not_uppercase(c: char) -> bool {
    let mut lowered = c.to_lowercase();
    lowered.next() == Some(c) && lowered.next().is_none()
}

fn lowercase_identifier(src: &str) -> Result<&str> {
    start_and_continue(
        src,
        |c| UnicodeXID::is_xid_start(c) && char_is_not_uppercase(c),
        UnicodeXID::is_xid_continue,
    )
}

fn uppercase_identifier(src: &str) -> Result<&str> {
    start_and_continue(
        src,
        |c| UnicodeXID::is_xid_start(c) && !char_is_not_uppercase(c),
        UnicodeXID::is_xid_continue,
    )
}

fn skip_whitespace(src: &str) -> &str {
    let mut rest = src;
    let mut cs = src.chars();
    loop {
        let c = cs.next();
        if some_char_is(c, char::is_whitespace) {
            rest = cs.as_str();
        } else if c == Some('#') {
            rest = cs.as_str();
            while some_char_is(cs.next(), |c| c != '\n') {
                rest = cs.as_str();
            }
        } else {
            break;
        }
    }
    return rest;
}

struct ParseContext<'d, 'c, D: 'd + Diagram> {
    diagram: &'d mut D,
    context: &'c mut Context,
}

fn arm<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
) -> Result<'a, Option<NodeIndex>> {
    let rest = skip_whitespace(src);
    let (_, rest) = character(rest, '{')?;
    let rest = skip_whitespace(rest);
    if let Ok((_, rest)) = character(rest, '}') {
        return Ok((None, rest));
    }
    if let Ok((name, rest)) = lowercase_identifier(rest) {
        let rest = skip_whitespace(rest);
        if let Ok((_, rest)) = character(rest, '}') {
            return Ok((
                Some(
                    context
                        .context
                        .reserve_node_name(name, context.diagram)
                        .index,
                ),
                rest,
            ));
        }
    }
    let (node_index, rest) = node(rest, context)?;
    let rest = skip_whitespace(rest);
    let (_, rest) = character(rest, '}')?;
    return Ok((Some(node_index), rest));
}

fn reserve_predicate<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
    parsed_predicate: ParsedPredicate<'a>,
    num_terms: usize,
) -> Result<'a, Predicate> {
    let predicate = match parsed_predicate {
        ParsedPredicate::Name(predicate_name) => context.context.reserve_predicate(predicate_name),
        ParsedPredicate::Number(predicate) => Predicate(predicate),
    };
    if let Some(num_terms) = context.context.get_num_terms_for_predicate(predicate) {
        if num_terms != num_terms {
            return err_msg("Wrong number of terms for predicate", src);
        }
    } else {
        context
            .context
            .num_terms_for_predicate
            .insert(predicate, num_terms);
    }
    Ok((predicate, src))
}

enum ParsedPredicate<'a> {
    Name(&'a str),
    Number(u64),
}

fn parse_predicate<'a, 'b, D: Diagram>(
    src: &'a str,
    _context: &'b mut ParseContext<D>,
) -> Result<'a, ParsedPredicate<'a>> {
    let rest = skip_whitespace(src);
    if let Ok((name, rest)) = lowercase_identifier(rest) {
        Ok((ParsedPredicate::Name(name), rest))
    } else if let Ok((_, rest)) = character(rest, '@') {
        let (number, rest) = unsigned_decimal_integer(rest)?;
        Ok((ParsedPredicate::Number(number), rest))
    } else {
        err_msg("Not a predicate", src)
    }
}

fn output_node<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
    name: Option<&'a str>,
) -> Result<'a, NodeIndex> {
    let rest = prefix(src, "output")?;
    let rest = skip_whitespace(rest);
    let (predicate, rest) = parse_predicate(rest, context)?;
    let rest = skip_whitespace(rest);
    let (terms, rest) = output_terms(rest, context)?;
    let predicate = reserve_predicate(src, context, predicate, terms.len())?.0;
    let node = Node::Output { predicate, terms };
    let node_index;
    if let Some(name) = name {
        let NodeInfo { defined, index } = context.context.reserve_node_name(name, context.diagram);
        node_index = index;
        if defined {
            return err_msg("Node with this name was already defined", src);
        }
        *context.diagram.get_node_mut(index) = node;
        if name == "root" {
            context.diagram.set_root(index);
        }
    } else {
        node_index = context.diagram.insert_node(node);
    }
    Ok((node_index, rest))
}

fn match_node<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
    name: Option<&'a str>,
) -> Result<'a, NodeIndex> {
    let (predicate, rest) = parse_predicate(src, context)?;
    let (terms, rest) = match_terms(rest, context)?;
    let (match_target, rest) = arm(rest, context)?;
    let (refute_target, rest) = if let Ok((t, r)) = arm(rest, context) {
        (t, r)
    } else {
        (None, rest)
    };
    let predicate = reserve_predicate(src, context, predicate, terms.len())?.0;
    let node = Node::Match { predicate, terms };
    if let Some(name) = name {
        let NodeInfo { defined, index } = context.context.reserve_node_name(name, context.diagram);
        if defined {
            return err_msg("Node with this name was already defined", src);
        }
        *context.diagram.get_node_mut(index) = node;
        if let Some(on_match) = match_target {
            context.diagram.set_on_match(index, on_match);
        }
        if let Some(on_refute) = refute_target {
            context.diagram.set_on_refute(index, on_refute);
        }
        if name == "root" {
            context.diagram.set_root(index);
        }
        Ok((index, rest))
    } else {
        let index = context.diagram.insert_node(node);
        if let Some(on_match) = match_target {
            context.diagram.set_on_match(index, on_match);
        }
        if let Some(on_refute) = refute_target {
            context.diagram.set_on_refute(index, on_refute);
        }
        Ok((index, rest))
    }
}

fn node_without_name<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
    name: Option<&'a str>,
) -> Result<'a, NodeIndex> {
    let rest = skip_whitespace(src);
    if let Ok((node, rest)) = output_node(rest, context, name) {
        return Ok((node, rest));
    };
    return match_node(src, context, name);
}

fn node<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
) -> Result<'a, NodeIndex> {
    let rest = skip_whitespace(src);
    if let Ok((name, rest)) = node_name(rest, context) {
        node_without_name(rest, context, Some(name))
    } else {
        node_without_name(rest, context, None)
    }
}

fn node_name<'a, 'b, D: Diagram>(
    src: &'a str,
    _context: &'b mut ParseContext<D>,
) -> Result<'a, &'a str> {
    let (name, rest) = lowercase_identifier(src)?;
    let rest = skip_whitespace(rest);
    let rest = character(rest, ':')?.1;
    Ok((name, rest))
}

fn arg_list<'a, I, F: FnMut(&'a str) -> Result<'a, I>>(
    src: &'a str,
    mut f: F,
) -> Result<'a, Vec<I>> {
    let rest = skip_whitespace(src);
    let (_, mut rest) = character(rest, '(')?;
    let mut items = Vec::new();
    loop {
        rest = skip_whitespace(rest);
        if let Ok((item, r)) = f(rest) {
            items.push(item);
            rest = r;
        } else {
            break;
        }
        rest = skip_whitespace(rest);
        if let Ok((_, r)) = character(rest, ',') {
            rest = r;
        } else {
            break;
        }
    }
    rest = skip_whitespace(rest);
    let (_, rest) = character(rest, ')')?;
    return Ok((items, rest));
}

fn match_terms<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
) -> Result<'a, Vec<MatchTerm>> {
    arg_list(src, |s| match_term(s, context))
}

fn match_term<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
) -> Result<'a, MatchTerm> {
    let mut rest = skip_whitespace(src);
    let constraint;
    if let Ok((_, r)) = character(rest, '_') {
        constraint = MatchTermConstraint::Free;
        rest = r;
    } else if let Ok((reg, r)) = register(rest, context) {
        constraint = MatchTermConstraint::Register(reg);
        rest = r;
    } else if let Ok((v, r)) = value(rest, context) {
        constraint = MatchTermConstraint::Constant(v);
        rest = r;
    } else {
        return err_msg("could not parse match term", src);
    }
    let mut rest = skip_whitespace(rest);
    let mut target = None;
    if let Ok(r) = prefix(rest, "->") {
        rest = skip_whitespace(r);
        let (reg, r) = register(rest, context)?;
        target = Some(reg);
        rest = r;
    }
    Ok((MatchTerm { constraint, target }, rest))
}

fn output_terms<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
) -> Result<'a, Vec<OutputTerm>> {
    arg_list(src, |s| output_term(s, context))
}

fn output_term<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
) -> Result<'a, OutputTerm> {
    let rest = skip_whitespace(src);
    if let Ok((reg, rest)) = register(rest, context) {
        Ok((OutputTerm::Register(reg), rest))
    } else if let Ok((v, rest)) = value(rest, context) {
        Ok((OutputTerm::Constant(v), rest))
    } else {
        err_msg("could not parse match term", src)
    }
}

fn register<'a, 'b, D: Diagram>(
    src: &'a str,
    _context: &'b mut ParseContext<D>,
) -> Result<'a, usize> {
    let rest = skip_whitespace(src);
    let (_, rest) = character(rest, '%')?;
    let (reg, rest) = unsigned_decimal_integer(rest)?;
    Ok((reg as usize, rest))
}

fn value<'a, 'b, D: Diagram>(src: &'a str, _context: &'b mut ParseContext<D>) -> Result<'a, Value> {
    let rest = skip_whitespace(src);
    let (_, rest) = character(rest, ':')?;
    let (symbol, rest) = unsigned_decimal_integer(rest)?;
    Ok((Value::Symbol(symbol), rest))
}

fn parse_diagram_inner<'a, 'b, D: Diagram>(
    src: &'a str,
    context: &'b mut ParseContext<D>,
) -> Result<'a, ()> {
    let mut rest = src;
    while rest != "" {
        let (_, r) = node(rest, context)?;
        rest = skip_whitespace(r);
    }
    Ok(((), rest))
}

pub fn parse_diagram(
    src: &str,
    num_registers: usize,
) -> std::result::Result<(GraphDiagram, Context), Error> {
    let mut d = GraphDiagram::new(num_registers);
    let mut c = Context::new();
    let result;
    {
        let mut context = ParseContext {
            diagram: &mut d,
            context: &mut c,
        };
        result = parse_diagram_inner(src, &mut context);
    }
    match result {
        Ok(_) => Ok((d, c)),
        Err(e) => Err(e),
    }
}

pub fn update_diagram<'a, 'b, 'c, D: Diagram>(
    src: &'a str,
    diagram: &'b mut D,
    context: &'a mut Context,
) -> std::result::Result<(), Error<'a>> {
    let result;
    {
        let mut context = ParseContext { diagram, context };
        result = parse_diagram_inner(src, &mut context);
    }
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_value() {
        let mut diagram = GraphDiagram::new(0);
        let mut context = Context::new();
        let mut c = ParseContext {
            diagram: &mut diagram,
            context: &mut context,
        };
        assert_eq!(value(":0", &mut c), Ok((Value::Symbol(0), "")));
        assert_eq!(value(":1", &mut c), Ok((Value::Symbol(1), "")));
        assert_eq!(
            value(":blank", &mut c),
            Err(Error::Msg {
                msg: "Wrong starting character",
                rest: "blank",
            })
        );
    }

    #[test]
    fn can_parse_register() {
        let mut diagram = GraphDiagram::new(0);
        let mut context = Context::new();
        let mut c = ParseContext {
            diagram: &mut diagram,
            context: &mut context,
        };
        assert_eq!(register("%0", &mut c), Ok((0, "")));
        assert_eq!(register("%1", &mut c), Ok((1, "")));
        assert_eq!(
            register("%test", &mut c),
            Err(Error::Msg {
                msg: "Wrong starting character",
                rest: "test",
            })
        );
    }

    #[test]
    fn can_parse_match_term() {
        let mut diagram = GraphDiagram::new(0);
        let mut context = Context::new();
        let mut c = ParseContext {
            diagram: &mut diagram,
            context: &mut context,
        };
        assert_eq!(
            match_term("_", &mut c),
            Ok((
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: None,
                },
                ""
            ))
        );
        assert_eq!(
            match_term("_->%1", &mut c),
            Ok((
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(1),
                },
                ""
            ))
        );
        assert_eq!(
            match_term("_ -> %1", &mut c),
            Ok((
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(1),
                },
                ""
            ))
        );
        assert_eq!(
            match_term(":2 -> %3", &mut c),
            Ok((
                MatchTerm {
                    constraint: MatchTermConstraint::Constant(Value::Symbol(2)),
                    target: Some(3),
                },
                ""
            ))
        );
        assert_eq!(
            match_term("%2 -> %3", &mut c),
            Ok((
                MatchTerm {
                    constraint: MatchTermConstraint::Register(2),
                    target: Some(3),
                },
                ""
            ))
        );
    }

    #[test]
    fn can_parse_match_terms() {
        let mut diagram = GraphDiagram::new(0);
        let mut context = Context::new();
        let mut c = ParseContext {
            diagram: &mut diagram,
            context: &mut context,
        };
        assert_eq!(
            match_terms(" ( _ ) ", &mut c),
            Ok((
                vec![
                    MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: None,
                    },
                ],
                " "
            ))
        );
        assert_eq!(
            match_terms(" ( _ , _ ) ", &mut c),
            Ok((
                vec![
                    MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: None,
                    },
                    MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: None,
                    },
                ],
                " "
            ))
        );
        assert_eq!(
            match_terms(" ( _ , _ , ) ", &mut c),
            Ok((
                vec![
                    MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: None,
                    },
                    MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: None,
                    },
                ],
                " "
            ))
        );
        assert_eq!(
            match_terms(" ( _ , _ ,, ) ", &mut c),
            Err(Error::Msg {
                msg: "Wrong character",
                rest: ", ) ",
            })
        );
        assert_eq!(
            match_terms(" ( _ -> %1 )", &mut c),
            Ok((
                vec![
                    MatchTerm {
                        constraint: MatchTermConstraint::Free,
                        target: Some(1),
                    },
                ],
                ""
            ))
        );
        assert_eq!(
            match_terms("(:2 -> %3,)", &mut c),
            Ok((
                vec![
                    MatchTerm {
                        constraint: MatchTermConstraint::Constant(Value::Symbol(2)),
                        target: Some(3),
                    },
                ],
                ""
            ))
        );
        assert_eq!(
            match_terms("(%2 -> %3)", &mut c),
            Ok((
                vec![
                    MatchTerm {
                        constraint: MatchTermConstraint::Register(2),
                        target: Some(3),
                    },
                ],
                ""
            ))
        );
    }

    #[test]
    fn can_parse_arm() {
        let mut diagram = GraphDiagram::new(0);
        let mut context = Context::new();
        let mut c = ParseContext {
            diagram: &mut diagram,
            context: &mut context,
        };
        assert_eq!(arm(" {  } ", &mut c), Ok((None, " ")));
        assert_eq!(c.diagram.len(), 0);
        assert_eq!(arm(" { test } ", &mut c), Ok((Some(NodeIndex(0)), " ")));
        assert_eq!(c.diagram.len(), 1);
        assert_eq!(arm(" { test } ", &mut c), Ok((Some(NodeIndex(0)), " ")));
        assert_eq!(c.diagram.len(), 1);
        assert_eq!(arm(" { retest } ", &mut c), Ok((Some(NodeIndex(1)), " ")));
        assert_eq!(c.diagram.len(), 2);
    }

    #[test]
    fn can_parse_single_node_diagram() {
        let mut expected_diagram = GraphDiagram::new(0);
        let output_node = Node::Output {
            predicate: Predicate(0),
            terms: vec![
                OutputTerm::Constant(Value::Symbol(1)),
                OutputTerm::Constant(Value::Symbol(2)),
            ],
        };
        let root = expected_diagram.insert_node(output_node);
        expected_diagram.set_root(root);
        let mut d = GraphDiagram::new(0);
        let mut context = Context::new();
        let mut c = ParseContext {
            diagram: &mut d,
            context: &mut context,
        };
        assert_eq!(
            parse_diagram_inner("root: output test(:1, :2)", &mut c),
            Ok(((), ""))
        );
        assert_eq!(c.diagram, &expected_diagram);
    }

    #[test]
    fn can_parse_nested_diagram() {
        let mut expected_diagram = GraphDiagram::new(2);
        let match_ones_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm {
                    constraint: MatchTermConstraint::Constant(Value::Symbol(1)),
                    target: Some(0),
                },
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(1),
                },
            ],
        };
        let match_anything_node = Node::Match {
            predicate: Predicate(0),
            terms: vec![
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: None,
                },
                MatchTerm {
                    constraint: MatchTermConstraint::Free,
                    target: Some(1),
                },
            ],
        };
        let output_node = Node::Output {
            predicate: Predicate(1),
            terms: vec![OutputTerm::Register(0), OutputTerm::Register(1)],
        };
        let output = expected_diagram.insert_node(output_node);
        let anything = expected_diagram.insert_node(match_anything_node);
        let root = expected_diagram.insert_node(match_ones_node);
        expected_diagram.set_root(root);
        expected_diagram.set_on_match(root, anything);
        expected_diagram.set_on_match(anything, output);

        let mut d = GraphDiagram::new(2);
        let mut context = Context::new();
        context
            .predicate_name_to_predicate
            .insert("a".to_owned(), Predicate(0));
        context
            .predicate_name_to_predicate
            .insert("b".to_owned(), Predicate(1));
        let mut c = ParseContext {
            diagram: &mut d,
            context: &mut context,
        };
        assert_eq!(
            parse_diagram_inner(
                r#"
                  root: a(:1 -> %0, _ -> %1) {
                    a(_, _ -> %1) {
                      output b(%0, %1)
                    }
                  }
                  "#,
                &mut c
            ),
            Ok(((), ""))
        );
        println!("parsed = {:#?}", c.diagram);
        println!("expected = {:#?}", expected_diagram);
        assert_eq!(c.diagram, &expected_diagram);
    }

    #[test]
    fn can_parse_explicit_diagram() {
        let mut expected_diagram = GraphDiagram::new(0);
        let output_node = Node::Output {
            predicate: Predicate(2),
            terms: vec![
                OutputTerm::Constant(Value::Symbol(1)),
                OutputTerm::Constant(Value::Symbol(2)),
            ],
        };
        let root = expected_diagram.insert_node(output_node);
        expected_diagram.set_root(root);
        let mut d = GraphDiagram::new(0);
        let mut context = Context::new();
        let mut c = ParseContext {
            diagram: &mut d,
            context: &mut context,
        };
        assert_eq!(
            parse_diagram_inner("root: output @2(:1, :2)", &mut c),
            Ok(((), ""))
        );
        assert_eq!(c.diagram, &expected_diagram);
    }
}
