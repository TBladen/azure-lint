mod parser;

use crate::rules::{ Property, Selector };
use reqwest;
use serde_json::Value;
use std::convert::TryFrom;

#[derive(Clone, Debug, PartialEq)]
pub struct Id {
  subscription_id: String,
  resource_group: String,
  kind: String,
  name: String,
}

impl TryFrom<&str> for Id {
  type Error = &'static str;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    if let Some((subscription_id, resource_group, provider, kind, name)) = parser::parse_id(value) {
      Ok(Id {
        subscription_id: subscription_id.to_owned(),
        resource_group: resource_group.to_owned(),
        kind: translate_kind(&format!("{}/{}", provider, kind)).to_owned(),
        name: name.to_owned(),
      })
    } else {
      Err("Failed to parse Azure identifier")
    }
  }
}

fn translate_kind(kind: &str) -> &str {
  match kind {
    "Microsoft.Web/serverFarms" => "app_service_plan",
    "Microsoft.Web/sites" => "app_service",
    _ => kind,
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resource(Id, serde_json::Value);

impl Resource {
  pub fn id(&self) -> &Id {
    &self.0
  }

  pub fn name(&self) -> &str {
    &self.id().name
  }

  pub fn kind(&self) -> &str {
    &self.id().kind
  }

  pub fn group(&self) -> &str {
    &self.id().resource_group
  }

  pub fn get_property(&self, property: &Property) -> Value {
    match property {
      Property::Name => self.name().into(),
      Property::Kind => self.kind().into(),
      Property::Group => self.group().into(),
      Property::Custom(key) => self.1[key].clone()
    }
  }

  pub fn selector_applies(&self, selector: &Selector) -> bool {
    (selector.cloud == "azure" || selector.cloud == "*") &&
    (selector.group == self.group() || selector.group == "*") &&
    (selector.kind == self.kind() || selector.kind == "*") &&
    (selector.name == self.name() || selector.name == "*")
  }
}

// Not all JSON objects are Azure resources
impl TryFrom<Value> for Resource {
  type Error = &'static str;

  fn try_from(value: Value) -> Result<Self, Self::Error> {
    if let Some(Ok(id)) = value["id"].as_str().map(Id::try_from) {
      Ok(Self(id, value))
    } else {
      Err("Not a valid Azure Resource Manager object")
    }
  }
}

// impl Index<&str> for Resource {
//     type Output = Value;

//     fn index(&self, index: &str) -> &Self::Output {
//       &self.1[index]
//     }
// }


pub struct Client {
  pub client: reqwest::blocking::Client,
  pub bearer_token: String,
}

impl Client {
  pub fn new(tenant_id: &str, client_id: &str, client_secret: &str) -> Client {
    let client = reqwest::blocking::Client::new();
    let bearer_token = get_bearer_token(&client, tenant_id, client_id, client_secret);

    Client {
      client: client,
      bearer_token: bearer_token,
    }
  }

  pub fn get_subscriptions(&self) -> Vec<String> {
    let res = self
      .client
      .get("https://management.azure.com/subscriptions")
      .query(&[("api-version", "2016-06-01")])
      .bearer_auth(self.bearer_token.to_owned())
      .send()
      .unwrap();

    let json = res.json::<Value>().unwrap();

    json["value"]
      .as_array()
      .unwrap()
      .iter()
      .map(|v| v["subscriptionId"].as_str().unwrap().to_owned())
      .collect()
  }

  pub fn get_resource_groups(&self, subscription_id: &str) -> Vec<String> {
    let url = format!(
      "https://management.azure.com/subscriptions/{}/resourcegroups",
      subscription_id
    );
    let res = self
      .client
      .get(&url)
      .query(&[("api-version", "2019-10-01")])
      .bearer_auth(self.bearer_token.to_owned())
      .send()
      .unwrap();

    let json = res.json::<Value>().unwrap();

    json["value"]
      .as_array()
      .unwrap()
      .iter()
      .map(|v| v["name"].as_str().unwrap().to_owned())
      .collect()
  }

  pub fn get_resources(&self, subscription_id: &str, resource_group_name: &str) -> Vec<Resource> {
    let url = format!(
      "https://management.azure.com/subscriptions/{}/resourceGroups/{}/resources",
      subscription_id, resource_group_name
    );
    let res = self
      .client
      .get(&url)
      .query(&[("api-version", "2019-10-01")])
      .bearer_auth(self.bearer_token.to_owned())
      .send()
      .unwrap();

    let json = res.json::<Value>().unwrap();

    json["value"].as_array()
      .map(|arr| {
        arr.iter()
          .filter_map(|r| Resource::try_from(r.clone()).ok())
          .collect()
      })
      .unwrap()
  }
}

fn get_bearer_token(
  client: &reqwest::blocking::Client,
  tenant_id: &str,
  client_id: &str,
  client_secret: &str,
) -> String {
  let token_endpoint = format!("https://login.windows.net/{}/oauth2/token", tenant_id);
  let body = format!("grant_type=client_credentials&client_id={}&resource=https%3A%2F%2Fmanagement.core.windows.net%2F&client_secret={}",
        client_id,
        client_secret
    );

  let res = client.post(&token_endpoint).body(body).send().unwrap();

  let json: Value = res.json().unwrap();
  json["access_token"].as_str().unwrap().to_owned()
}
