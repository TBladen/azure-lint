use nom::{
  bytes::complete::take_until,
  bytes::complete::take_while1,
  bytes::complete::tag,
  combinator::all_consuming,
  sequence::tuple,
  IResult
};

type ParseResult<'a, T> = IResult<&'a str, T>;

fn subscription_id(i: &str) -> ParseResult<&str> {
  take_until("/")(i)
}

fn resource_group(i: &str) -> ParseResult<&str> {
  take_until("/")(i)
}

fn provider(i: &str) -> ParseResult<&str> {
  take_until("/")(i)
}

fn kind(i: &str) -> ParseResult<&str> {
  take_until("/")(i)
}

fn name(i: &str) -> ParseResult<&str> {
    take_while1(|c| char::is_alphanumeric(c) || c == '-' || c == '-' || c == ' ')(i)
}

pub fn parse_id(i: &str) -> Option<(&str, &str, &str, &str, &str)> {
    // /subscriptions/00d88f1a-26e6-4665-9eee-00359b7f1717/resourceGroups/test-group/providers/Microsoft.Storage/storageAccounts/ihbtesting123
    let parser = tuple((tag("/subscriptions/"), subscription_id, tag("/resourceGroups/"), resource_group, tag("/providers/"), provider, tag("/"), kind, tag("/"), name));
    let (_, (_, subscription_id, _, resource_group, _, provider, _, kind, _, name)) = all_consuming(parser)(i).unwrap();

    Some((subscription_id, resource_group, provider, kind, name))
}
