use strum::{EnumString, AsRefStr, IntoStaticStr, Display};

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

