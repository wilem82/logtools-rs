use either::{Left, Right};

fn unwrap_or_empty<L, I>(opt_iter: Option<L>) -> either::Either<L, std::iter::Empty<I>>
where
    L: Iterator<Item = I>
{
    match opt_iter {
        Some(it) => Left(it),
        None => Right(std::iter::empty()),
    }
}


pub fn parse_matchers(cli: &clap::ArgMatches, verbatim_name: &str, regex_name: &str) -> Vec<Box<dyn Matcher>> {
    let verbatims = unwrap_or_empty(cli.values_of(verbatim_name))
        .map(|it| VerbatimMatcher(it.to_string()))
        .map(|it| Box::new(it) as Box<dyn Matcher>);
    let regexes = unwrap_or_empty(cli.values_of(regex_name))
        .map(|it| RegexMatcher(regex::Regex::new(it).unwrap()))
        .map(|it| Box::new(it) as Box<dyn Matcher>);
    verbatims.chain(regexes).collect::<Vec<Box<dyn Matcher>>>()
}

pub trait Matcher {
    fn matches<'a>(&self, s: &'a str) -> bool;
}

pub struct VerbatimMatcher(String);
pub struct RegexMatcher(regex::Regex);

impl Matcher for VerbatimMatcher {
    fn matches<'a>(&self, s: &'a str) -> bool {
        s.contains(self.0.as_str())
    }
}

impl Matcher for RegexMatcher {
    fn matches<'a>(&self, s: &'a str) -> bool {
        self.0.is_match(s)
    }
}
