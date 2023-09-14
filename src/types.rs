use std::{
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

use tree_sitter::{Point, Node};

use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

use itertools::Itertools;

#[allow(dead_code)]
pub struct RSymbolV2<'a> {
    pub kind: RSymbolKind,
    pub name: String,
    pub scope: Scope,
    pub file: PathBuf,
    pub node: Weak<Node<'a>>,
    pub start: Point,
    pub end: Point,
    pub parent: Option<Arc<RSymbolV2<'a>>>
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
pub enum RSymbolKind {
    Class { superclass_scope: Scope }, 
    Module { superclass_scope: Scope },
    InstanceMethod { parameters: Vec<RMethodParamV2> },
    SingletonMethod { parameters: Vec<RMethodParamV2> },
    Constant,
    Variable,
    InstanceVariable,
    ClassVariable,
    GlobalVariable
}

impl RSymbolKind {
    pub fn is_classlike(&self) -> bool {
        match self {
            RSymbolKind::Class { .. } => true,
            RSymbolKind::Module { .. } => true,
            _ => false
        }
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Eq)]
pub enum RSymbol {
    Class(RClass),
    Module(RClass),
    Method(RMethod),
    SingletonMethod(RMethod),
    Constant(RConstant),
    Variable(RVariable),
    GlobalVariable(RVariable),
    ClassVariable(RVariable),
}

impl RSymbol {
    pub fn kind(&self) -> &str {
        match self {
            RSymbol::Class(_) => "class",
            RSymbol::Module(_) => "module",
            RSymbol::Method(_) => "method",
            RSymbol::SingletonMethod(_) => "singleton_method",
            RSymbol::Constant(_) => "constant",
            RSymbol::Variable(_) => "variable",
            RSymbol::GlobalVariable(_) => "global_variable",
            RSymbol::ClassVariable(_) => "class_variable",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            RSymbol::Class(class) => &class.name,
            RSymbol::Module(module) => &module.name,
            RSymbol::Method(method) => &method.name,
            RSymbol::SingletonMethod(method) => &method.name,
            RSymbol::Constant(constant) => &constant.name,
            RSymbol::Variable(variable) => &variable.name,
            RSymbol::GlobalVariable(variable) => &variable.name,
            RSymbol::ClassVariable(variable) => &variable.name,
        }
    }

    pub fn full_scope(&self) -> &Scope {
        match self {
            RSymbol::Class(s) => &s.scope,
            RSymbol::Module(s) => &s.scope,
            RSymbol::Method(s) => &s.scope,
            RSymbol::SingletonMethod(s) => &s.scope,
            RSymbol::Constant(s) => &s.scope,
            RSymbol::Variable(s) => &s.scope,
            RSymbol::GlobalVariable(s) => &s.scope,
            RSymbol::ClassVariable(s) => &s.scope,
        }
    }

    pub fn file(&self) -> &Path {
        match self {
            RSymbol::Class(class) => &class.file,
            RSymbol::Module(module) => &module.file,
            RSymbol::Method(method) => &method.file,
            RSymbol::SingletonMethod(method) => &method.file,
            RSymbol::Constant(constant) => &constant.file,
            RSymbol::Variable(variable) => &variable.file,
            RSymbol::GlobalVariable(variable) => &variable.file,
            RSymbol::ClassVariable(v) => &v.file,
        }
    }

    pub fn location(&self) -> &Point {
        match self {
            RSymbol::Class(class) => &class.location,
            RSymbol::Module(module) => &module.location,
            RSymbol::Method(method) => &method.location,
            RSymbol::SingletonMethod(method) => &method.location,
            RSymbol::Constant(constant) => &constant.location,
            RSymbol::Variable(variable) => &variable.location,
            RSymbol::GlobalVariable(variable) => &variable.location,
            RSymbol::ClassVariable(variable) => &variable.location,
        }
    }

    pub fn parent(&self) -> &Option<Arc<RSymbol>> {
        match self {
            RSymbol::Class(s) => &s.parent,
            RSymbol::Module(s) => &s.parent,
            RSymbol::Method(s) => &s.parent,
            RSymbol::SingletonMethod(s) => &s.parent,
            RSymbol::Constant(s) => &s.parent,
            RSymbol::Variable(s) => &s.parent,
            RSymbol::GlobalVariable(s) => &s.parent,
            RSymbol::ClassVariable(s) => &s.parent,
        }
    }
}

impl std::fmt::Debug for RSymbolV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} in {:?} at {:?}, name = {}, scope = {}, parent = {:?}",
            self.kind,
            self.file,
            self.start,
            self.name,
            self.scope,
            self.parent
        )
    }
}

impl std::fmt::Debug for RSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} in {:?} at {:?}, name = {}, parent = {:?}",
            self.kind(),
            self.file(),
            self.location(),
            self.name(),
            self.parent()
        )
    }
}

#[derive(PartialEq, Eq)]
pub struct RClass {
    pub file: PathBuf,
    pub name: String,
    pub scope: Scope,
    pub location: Point,
    pub superclass_scopes: Scope,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq)]
pub struct RMethod {
    pub file: PathBuf,
    pub name: String,
    pub scope: Scope,
    pub location: Point,
    pub parameters: Vec<RMethodParam>,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum RMethodParam {
    Regular(MethodParam),
    Optional(MethodParam),
    Keyword(MethodParam),
}

#[derive(PartialEq, Eq, Debug)]
pub enum RMethodParamKind {
    Regular, Optional, Keyword
}

#[derive(PartialEq, Eq, Debug)]
pub struct RMethodParamV2 {
    pub kind: RMethodParamKind,
    pub file: PathBuf,
    pub name: String,
    pub start: Point,
    pub end: Point,
}

#[derive(PartialEq, Eq, Debug)]
pub struct MethodParam {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
}

#[derive(PartialEq, Eq)]
pub struct RConstant {
    pub file: PathBuf,
    pub name: String,
    pub scope: Scope,
    pub location: Point,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq)]
pub struct RVariable {
    pub file: PathBuf,
    pub name: String,
    pub scope: Scope,
    pub location: Point,
    pub parent: Option<Arc<RSymbol>>,
}

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
    #[strum(serialize = "self")]
    Zelf,
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
    Arguments,
}

impl AsRef<[u8]> for NodeName {
    fn as_ref(&self) -> &[u8] {
        Into::<&str>::into(self).as_bytes()
    }
}



#[derive(PartialEq, Eq, Debug, Clone)]
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

    pub fn without_last(&self) -> Scope {
        let mut scopes = self.scopes.clone();
        scopes.pop();
        Scope::new(scopes)
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

impl PartialEq<Scope> for &[&str] {
    fn eq(&self, other: &Scope) -> bool {
        let s: Vec<&str> = other.scopes.iter().map(|s| s.as_str()).collect();

        *self == s
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

