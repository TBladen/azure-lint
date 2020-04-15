use crate::rules::{ Condition, Property, Rule, Selector };
use nom::branch::alt;
use nom::bytes::complete::{ tag, take_until, take_while1 };
use nom::character::complete::{ alpha1, multispace1, space1 };
use nom::combinator::{ all_consuming, opt, peek };
use nom::multi::{ many0_count, separated_list };
use nom::sequence::tuple;
use nom::IResult;
use regex::Regex;
use std::convert::{ AsRef, TryInto };
use std::path::Path;

// selector.selector.selector {
//     location = "uksouth"
//     name ~= /^[a-z]$/
// }

type ParseResult<'a, T> = IResult<&'a str, T>;

// Utility parsers
fn opening_brace(i: &str) -> ParseResult<&str> {
  tag("{")(i)
}

fn closing_brace(i: &str) -> ParseResult<&str> {
  tag("}")(i)
}

fn comment(i: &str) -> ParseResult<&str> {
  let parser = tuple((peek(tag("//")), take_until("\n")));
  let (rest, (_, comment)) = parser(i)?;

  Ok((rest, comment))
}

fn space_or_comment(i: &str) -> ParseResult<&str> {
  alt((multispace1, comment))(i)
}

// Grammer components
fn selector(i: &str) -> ParseResult<Selector> {
  let parser = take_while1(|c| char::is_alphanumeric(c) || c == '.' || c == '-' || c == '_' || c == '*'); 
  let (rest, selector) = parser(i)?;
  
  Ok((rest, selector.try_into().unwrap()))
}

fn property(i: &str) -> ParseResult<Property> {
  let (rest, property) = alpha1(i)?;

  Ok((rest, property.try_into().unwrap()))
}

fn value(i: &str) -> ParseResult<&str> {
  let parser = tuple((tag("\""), alpha1, tag("\"")));
  let (rest, (_, value, _)) = parser(i)?;

  Ok((rest, value))
}

fn equal_rule(i: &str) -> ParseResult<Condition> {
  let parser = tuple((tag("="), space1, value));
  let (rest, (_, _, value)) = parser(i)?;

  Ok((rest, Condition::Equal(value.to_owned())))
}

fn regex(i: &str) -> ParseResult<Regex> {
  let parser = tuple((tag("/"), take_until("/"), tag("/")));
  let (rest, (_, pattern, _)) = parser(i)?;

  // TODO: check if valid regex and return custom error if not
  Ok((rest, Regex::new(pattern).unwrap()))
}

fn match_rule(i: &str) -> ParseResult<Condition> {
  let parser = tuple((tag("~="), space1, regex));
  let (rest, (_, _, regex)) = parser(i)?;

  Ok((rest, Condition::Match(regex)))
}

fn rule_condition(i: &str) -> ParseResult<(Property, Condition)> {
  let parser = tuple((property, space1, alt((equal_rule, match_rule))));
  let (rest, (property, _, condition)) = parser(i)?;

  Ok((rest, (property, condition)))
}

fn rule_block_line_delim(i: &str) -> ParseResult<&str> {
  let parser = tuple((opt(comment), tag("\n"), many0_count(space_or_comment)));
  let (rest, _) = parser(i)?;

  Ok((rest, ""))
}

fn rule_block(i: &str) -> ParseResult<Vec<Rule>> {
  let rule_condition_lines = separated_list(rule_block_line_delim, rule_condition);
  let parser = tuple((selector, space1, opening_brace, multispace1, rule_condition_lines, multispace1, closing_brace));
  let (rest, (selector, _, _, _, conditions, _, _)) = parser(i)?;

  Ok((
    rest,
    conditions.iter()
      .map(
        |(property, condition)| Rule { selector: selector.clone(), property: property.clone(), condition: condition.clone() }
      )
      .collect()
  ))
}

fn flatten<T>(nested: Vec<Vec<T>>) -> Vec<T> {
  nested.into_iter().flatten().collect()
}

fn rule_blocks(i: &str) -> ParseResult<Vec<Rule>> {
  let parser = separated_list(rule_block_line_delim, rule_block);
  let (rest, rule_blocks) = parser(i)?;

  Ok((rest, flatten(rule_blocks)))
}

pub fn parse_rules(path: impl AsRef<Path>) -> Option<Vec<Rule>> {
  use std::fs::File;
  use std::io::prelude::*;

  let contents = {
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    contents
  };

  let parser = all_consuming(tuple((many0_count(space_or_comment), rule_blocks, many0_count(space_or_comment))));
  let (_, (_, rules, _)) = parser(&contents).unwrap();

  Some(rules)
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_opening_brace() {
    assert_eq!(opening_brace("{ hello"), Ok((" hello", "{")));
  }

  #[test]
  fn test_closing_brace() {
    assert_eq!(closing_brace("} hello"), Ok((" hello", "}")));
  }

  #[test]
  fn test_selector() {
    assert_eq!(
      selector("azure.test-rg"),
      Ok(("", "azure.test-rg".try_into().unwrap()))
    );

    assert_eq!(
      selector("azure.test_rg"),
      Ok(("", "azure.test_rg".try_into().unwrap()))
    );

    assert_eq!(
      selector("azure.test-rg-123"),
      Ok(("", "azure.test-rg-123".try_into().unwrap()))
    );

    assert_eq!(
      selector("azure.test_rg_123"),
      Ok(("", "azure.test_rg_123".try_into().unwrap()))
    );

  }

  #[test]
  fn test_equal_rule() {
    assert_eq!(
      equal_rule("= \"azure\""),
      Ok(("", Condition::Equal("azure".to_owned())))
    );
  }

  #[test]
  fn test_match_rule() {
    assert_eq!(
      match_rule("~= /^[a-z]+$/"),
      Ok(("", Condition::Match(Regex::new("^[a-z]+$").unwrap())))
    );
  }

  #[test]
  fn test_rule_condition() {
    assert_eq!(
      rule_condition("location = \"uksouth\""),
      Ok(("", ("location".try_into().unwrap(), Condition::Equal("uksouth".to_owned()))))
    );

    assert_eq!(
      rule_condition("name ~= /^[a-zA-Z0-9]+$/"),
      Ok(("", ("name".try_into().unwrap(), Condition::Match(Regex::new("^[a-zA-Z0-9]+$").unwrap()))))
    );
  }

  #[test]
  fn test_rule_block() {
    assert_eq!(
      rule_block("azure.test-rg { location = \"uksouth\" }"),
      Ok(("", vec![Rule {
        selector: "azure.test-rg".try_into().unwrap(),
        property: "location".try_into().unwrap(),
        condition: Condition::Equal("uksouth".to_owned())
      }]))
    );

    assert_eq!(
      rule_block("azure.test-rg {\n\tlocation = \"uksouth\"\n}"),
      Ok(("", vec![Rule {
        selector: "azure.test-rg".try_into().unwrap(),
        property: "location".try_into().unwrap(),
        condition: Condition::Equal("uksouth".to_owned())
      }]))
    );

    assert_eq!(
      rule_block("azure.test-rg {\n\tlocation = \"uksouth\"\n\tname ~= /^[a-zA-Z0-9]+$/\n}"),
      Ok(("", vec![
        Rule {
          selector: "azure.test-rg".try_into().unwrap(),
          property: "location".try_into().unwrap(),
          condition: Condition::Equal("uksouth".to_owned()),
        },
        Rule {
          selector: "azure.test-rg".try_into().unwrap(),
          property: "name".try_into().unwrap(),
          condition: Condition::Match(Regex::new("^[a-zA-Z0-9]+$").unwrap()),
        }
      ]))
    );
  }

}
