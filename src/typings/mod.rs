use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy)]
pub enum Type {
    Int,
    Bool,
    Void,
    Unresolved,
    Error,
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let type_name = match self {
            Type::Int => "int",
            Type::Bool => "bool",
            Type::Void => "void",
            Type::Unresolved => "unresolved",
            Type::Error => "?",
        };

        write!(f, "{type_name}")
    }
}

impl Type {
    pub fn is_assignable_to(self, other: Type) -> bool {
        matches!(
            (self, other),
            (Type::Int, Type::Int) | (Type::Bool, Type::Bool) | (Type::Error, _) | (_, Type::Error)
        )
    }

    pub fn from_str(s: &str) -> Option<Type> {
        match s {
            "int" => Some(Type::Int),
            "bool" => Some(Type::Bool),
            "void" => Some(Type::Void),
            _ => None,
        }
    }
}
