use regex::Regex;
use std::convert::TryFrom;
use std::fmt;

// Selector (cloud.group.type.name)
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Selector {
  pub cloud: String,
  pub group: String,
  pub kind: String,
  pub name: String,

  full_selector: String,
}

impl TryFrom<&'_ str> for Selector {
  type Error = &'static str;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    let parts: Vec<&str> = value.split('.').collect();

    if parts.len() > 0 {
      Ok(Self {
        cloud: parts.get(0).map(|s| s.to_string()).unwrap_or("*".to_owned()),
        group: parts.get(1).map(|s| s.to_string()).unwrap_or("*".to_owned()),
        kind: parts.get(2).map(|s| s.to_string()).unwrap_or("*".to_owned()),
        name: parts.get(3).map(|s| s.to_string()).unwrap_or("*".to_owned()),

        full_selector: value.to_string()
      })
    } else {
      Err("No selector parts")
    }
  }
}

impl TryFrom<String> for Selector {
  type Error = &'static str;

  fn try_from(value: String) -> Result<Self, Self::Error> {
    let ref_: &str = &value;
    
    TryFrom::try_from(ref_)
  }
}

impl TryFrom<&String> for Selector {
  type Error = &'static str;

  fn try_from(value: &String) -> Result<Self, Self::Error> {
    let ref_: &str = &value;
    
    TryFrom::try_from(ref_)
  }
}

// impl AsRef<String> for Selector {
//   fn as_ref(&self) -> &String {
//     &self.full_selector
//   }
// }

impl fmt::Display for Selector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.full_selector)
  }
}

// Property
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
  Name,
  Kind,
  Group,
  Custom(String),
}

impl TryFrom<&str> for Property {
  type Error = &'static str;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    Ok(match value {
      "name" => Self::Name,
      "type" => Self::Kind,
      "group" => Self::Group,
      _ => Self::Custom(value.to_owned()),
    })
  }
}

impl TryFrom<String> for Property {
  type Error = &'static str;

  fn try_from(value: String) -> Result<Self, Self::Error> {
    let ref_: &str = &value;
    
    TryFrom::try_from(ref_)
  }
}

impl TryFrom<&String> for Property {
  type Error = &'static str;

  fn try_from(value: &String) -> Result<Self, Self::Error> {
    let ref_: &str = &value;
    
    TryFrom::try_from(ref_)
  }
}

// Condition
#[derive(Debug, Clone)]
pub enum Condition {
  Equal(String),
  Match(Regex),
}

impl Condition {
  pub fn is_compliant(&self, value: &str) -> bool {
    match self {
      Self::Equal(expected) => expected == value,
      Self::Match(regex) => regex.is_match(value),
    }
  }
}

impl PartialEq for Condition {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Equal(a), Self::Equal(b)) => a == b,
      (Self::Match(a), Self::Match(b)) => a.as_str() == b.as_str(),
      _ => false
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
  pub selector: Selector,
  pub property: Property,
  pub condition: Condition,
}

impl fmt::Display for Rule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let (op, expected) = match &self.condition {
      Condition::Equal(x) => ("equal", x.to_owned()),
      Condition::Match(x) => ("match", format!("/{}/", x)),
    };

    write!(f, "Expected {:?} to {} {}", self.property, op, expected)
  }
}

// pub fn get_rules() -> Vec<Rule> {
//   let rules = vec![
//     Rule {
//       selector: "azure".try_into().unwrap(),
//       property: "location".try_into().unwrap(),
//       condition: Condition::Equal("uksouth".to_owned()),
//     },
//     Rule {
//       selector: "azure".try_into().unwrap(),
//       property: "name".try_into().unwrap(),
//       condition: Condition::Match(Regex::new(r"^[a-zA-Z0-9-]+$").unwrap()),
//     },
//   ];

//   rules
// }