use std::{path::{Path, PathBuf}, sync::Arc};

use tree_sitter::Point;

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
    pub location: Point,
    pub scopes: Vec<String>,
    pub superclass_scopes: Vec<String>,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq)]
pub struct RMethod {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub parameters: Vec<RMethodParam>,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq)]
pub enum RMethodParam {
    Regular(MethodParam),
    Optional(MethodParam),
    Keyword(MethodParam),
}

#[derive(PartialEq, Eq)]
pub struct MethodParam {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
}

#[derive(PartialEq, Eq)]
pub struct RConstant {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq)]
pub struct RVariable {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub parent: Option<Arc<RSymbol>>,
}

