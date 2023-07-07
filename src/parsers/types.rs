use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

use itertools::Itertools;

pub const SCOPE_DELIMITER: &str = "::";

pub const GLOBAL_SCOPE_VALUE: &str = "$GLOBAL";

#[derive(PartialEq, Eq, Debug, EnumString, AsRefStr, IntoStaticStr, Display)]
#[strum(serialize_all = "snake_case")]
pub enum NodeKind {
    Class,
    Module,
    Method,
    SingletonMethod,
    Assignment,
    Program,
    Comment,
    Call,
    Constant,
    LeftAssignmentList,
    GlobalVariable,
    ScopeResolution,
    ClassVariable,
    InstanceVariable,
    Identifier,
    RestAssignment,
    OptionalParameter,
    KeywordParameter,
}

impl PartialEq<NodeKind> for &str {
    fn eq(&self, other: &NodeKind) -> bool {
        let other: &str = other.into();
        (*self).eq(other)
    }
}

#[derive(PartialEq, Eq, Debug, EnumString, AsRefStr, IntoStaticStr, Display)]
#[strum(serialize_all = "snake_case")]
pub enum NodeName {
    Name,
    Superclass,
    Body,
    Scope,
    Left,
    Parameters,
    Receiver,
    Method,
}

impl AsRef<[u8]> for NodeName {
    fn as_ref(&self) -> &[u8] {
        Into::<&str>::into(self).as_bytes()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Scope {
    scopes: Vec<String>,
}

pub trait ScopeJoin<T> {
    fn join(&self, rhs: T) -> Scope;
}

impl Scope {
    pub fn new(scopes: Vec<String>) -> Scope {
        Scope {
            scopes,
        }
    }

    pub fn is_global(&self) -> bool {
        self.scopes.first().map(|s| s == GLOBAL_SCOPE_VALUE).unwrap_or(false)
    }

    pub fn join(&self, rhs: &Scope) -> Scope {
        let rhs = if rhs.is_global() { rhs.scopes.iter().skip(1) } else { rhs.scopes.iter().skip(0) };

        let new_scopes = self.scopes.iter().chain(rhs).cloned().collect();

        Scope::new(new_scopes)
    }

    pub fn last(&self) -> Option<&str> {
        self.scopes.last().map(|s| s.as_str())
    }

    pub fn remove_last(&mut self) {
        self.scopes.pop();
    }
}

impl From<String> for Scope {
    fn from(value: String) -> Self {
        Scope::new(vec![value])
    }
}

impl From<&String> for Scope {
    fn from(value: &String) -> Self {
        Scope::new(vec![value.to_owned()])
    }
}

impl From<&str> for Scope {
    fn from(value: &str) -> Self {
        Scope::new(vec![value.to_string()])
    }
}

impl From<Vec<String>> for Scope {
    fn from(value: Vec<String>) -> Self {
        Scope::new(value)
    }
}

impl From<Vec<&str>> for Scope {
    fn from(value: Vec<&str>) -> Self {
        let cloned = value.iter().map(|s| s.to_string()).collect();
        Scope::new(cloned)
    }
}

impl PartialEq<Vec<&str>> for Scope {
    fn eq(&self, other: &Vec<&str>) -> bool {
        let s: Vec<&str> = self.scopes.iter().map(|s| s.as_str()).collect();

        s == *other
    }
}

impl PartialEq<[&str]> for Scope {
    fn eq(&self, other: &[&str]) -> bool {
        let s: Vec<&str> = self.scopes.iter().map(|s| s.as_str()).collect();

        s == *other
    }
}

impl PartialEq<&[&str]> for Scope {
    fn eq(&self, other: &&[&str]) -> bool {
        let s: Vec<&str> = self.scopes.iter().map(|s| s.as_str()).collect();

        s == *other
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = if self.is_global() {
            self.scopes.iter().skip(1).join(SCOPE_DELIMITER)
        } else {
            self.scopes.join(SCOPE_DELIMITER)
        };
        write!(f, "{str}")
    }
}
