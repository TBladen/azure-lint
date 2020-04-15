
//
// TODO:
//   * Read rules from file (Y)
//   * Hierarchical rules (most specific rule is used for conflicts)
//   * Write JUNIT output
//   * regex match condition (Y)
//

mod azurerm;
mod parser;
mod rules;

use azurerm::Resource;
use rules::Rule;

use clap;

struct ResourceCompliance {
  resource_name: String,
  resource_type: String,

  compliant_rules: Vec<Rule>,
  noncompliant_rules: Vec<Rule>,
}

fn evaluate_rules(resource: &Resource, rules: &Vec<Rule>) -> ResourceCompliance {
  let resource_name = resource.name();
  let resource_kind = resource.kind();

  let mut compliant_rules: Vec<Rule> = Vec::new();
  let mut noncompliant_rules: Vec<Rule> = Vec::new();
  let mut nonapplicable_rules: Vec<Rule> = Vec::new();

  for rule in rules {
    if resource.selector_applies(&rule.selector) {
      let prop = resource.get_property(&rule.property);

      if let Some(value) = prop.as_str() {
        if rule.condition.is_compliant(value) {
          compliant_rules.push(rule.clone());
          continue;
        } 
      }

      noncompliant_rules.push(rule.clone());
    } else {
      println!(
        "{} {{ {:?} {:?} }} does not apply to {} in {} ({})",
        rule.selector, rule.property, rule.condition,
        resource.name(), resource.group(), resource.kind()
      );
      nonapplicable_rules.push(rule.clone());
    }
  }

  ResourceCompliance {
    resource_name: resource_name.to_owned(),
    resource_type: resource_kind.to_owned(),
    compliant_rules,
    noncompliant_rules,
  }
}

#[derive(Default)]
struct ResourceGroupCompliance {
  resource_count: usize,         // the total number of resources evaluated
  compliant_resources: usize,    // the number of completely compliant resources
  noncompliant_resources: usize, // the number of rules non-compliant with at least one rule

  evaluated_rules: usize, // the total number of rule evaluations (e.g. 1 rule * 3 resources = 3 evaluations)
  compliant_rule_evaluations: usize, // the total number of rules that evaluated as compliant
  noncompliant_rule_evaluations: usize, // the total number of rules that evaluated as noncompliant
}

fn accumulate_group_compliance(
  group_compliance: ResourceGroupCompliance,
  resource_compliance: &ResourceCompliance,
) -> ResourceGroupCompliance {
  let is_compliant = resource_compliance.noncompliant_rules.len() > 0;

  ResourceGroupCompliance {
    resource_count: group_compliance.resource_count + 1,
    compliant_resources: group_compliance.compliant_resources + (if is_compliant { 1 } else { 0 }),
    noncompliant_resources: group_compliance.noncompliant_resources
      + (if is_compliant { 0 } else { 1 }),

    evaluated_rules: group_compliance.evaluated_rules
      + resource_compliance.compliant_rules.len()
      + resource_compliance.noncompliant_rules.len(),
    compliant_rule_evaluations: group_compliance.compliant_rule_evaluations
      + resource_compliance.compliant_rules.len(),
    noncompliant_rule_evaluations: group_compliance.noncompliant_rule_evaluations
      + resource_compliance.noncompliant_rules.len(),
  }
}

#[derive(Debug)]
enum ClientLintError {
  CommandLineError,
  ParserError,
  CloudError,
}

type ApplicationResult = Result<(Vec<ResourceCompliance>, ResourceGroupCompliance), ClientLintError>;

fn azure_lint(rules: &Vec<Rule>, tenant_id: &str, client_id: &str, client_secret: &str) -> ApplicationResult {
  let client = azurerm::Client::new(tenant_id, client_id, client_secret);
  let subscriptions = client.get_subscriptions();
  let subscription_id = &subscriptions[0];

  let resource_groups = client.get_resource_groups(&subscription_id);
  let resource_group_name = &resource_groups[0];
  let resources = client.get_resources(subscription_id, resource_group_name);

  let compliance = resources
    .iter()
    .map(|r| evaluate_rules(&r, &rules))
    .collect::<Vec<ResourceCompliance>>();

  let group_compliance = compliance.iter().fold(
    ResourceGroupCompliance::default(),
    accumulate_group_compliance,
  );

  Ok((compliance, group_compliance))
}

fn main() -> Result<(), ClientLintError> {
  use clap::{App, Arg, SubCommand};

  let matches = App::new("cloud-lint")
    .version("0.1")
    .author("T. Bladen-Hovell")
    .about("Lint your cloud resources")
    .subcommand(
      SubCommand::with_name("azure")
        .about("Inspect an Azure resource group")
        .arg(Arg::with_name("FILE").index(1).required(true))
        .arg(Arg::with_name("tenant-id").long("tenant-id").takes_value(true).required(true))
        .arg(Arg::with_name("client-id").long("client-id").takes_value(true).required(true))
        .arg(Arg::with_name("client-secret").long("client-secret").takes_value(true).required(true)),
    )
    .get_matches();

  let (compliance, group_compliance) = match matches.subcommand() {
    ("azure", Some(subcmd)) => azure_lint(
      &parser::parse_rules(subcmd.value_of("FILE").ok_or(ClientLintError::CommandLineError)?).ok_or(ClientLintError::ParserError)?,
      subcmd.value_of("tenant-id").ok_or(ClientLintError::CommandLineError)?,
      subcmd.value_of("client-id").ok_or(ClientLintError::CommandLineError)?,
      subcmd.value_of("client-secret").ok_or(ClientLintError::CommandLineError)?,
    ),
    _ => Err(ClientLintError::CommandLineError),
  }?;

  println!(
    "Compliance score is {:.0}% ({}/{} rules compliant across {} resources",
    group_compliance.compliant_rule_evaluations as f64 / group_compliance.evaluated_rules as f64
      * 100.0,
    group_compliance.compliant_rule_evaluations,
    group_compliance.evaluated_rules,
    group_compliance.resource_count,
  );

  for resource in compliance {
    if resource.noncompliant_rules.len() > 0 {
      println!(
        "Resource {} ({}) is not compliant with the following rules:",
        resource.resource_name, resource.resource_type
      );

      for rule in resource.noncompliant_rules {
        println!("    {}", rule);
      }
    }
  }

  Ok(())
}
