use terebinth_span::{Symbol, sym};

use crate::attr::{self, AttributeExt};

#[derive(Debug)]
pub enum EntryPointType {
    None,
    MainNamed,
    TerebinthMainAttr,
    OtherMain,
}

pub fn entry_point_type(
    attrs: &[impl AttributeExt],
    at_root: bool,
    name: Option<Symbol>,
) -> EntryPointType {
    if attr::contains_name(attrs, sym::terebinth_main) {
        EntryPointType::TerebinthMainAttr
    } else if let Some(name) = name
        && name == sym::main
    {
        if at_root {
            EntryPointType::MainNamed
        } else {
            EntryPointType::OtherMain
        }
    } else {
        EntryPointType::None
    }
}
