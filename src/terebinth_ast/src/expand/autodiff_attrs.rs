use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use crate::expand::{Decodable, Encodable, HashStable_Generic};
use crate::ptr::P;
use crate::{Ty, TyKind};

#[derive(Clone, Copy, Eq, PartialEq, Encodable, Decodable, Debug, HashStable_Generic)]
pub enum DiffMode {
    Error,
    Source,
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Eq, PartialEq, Encodable, Decodable, Debug, HashStable_Generic)]
pub enum DiffActivity {
    None,
    Const,
    Active,
    ActiveOnly,
    Dual,
    DualOnly,
    Duplicated,
    DuplicatedOnly,
    FakeActivitySize,
}

#[derive(Clone, Eq, PartialEq, Encodable, Decodable, Debug, HashStable_Generic)]
pub struct AutoDiffItem {
    pub source: String,
    pub target: String,
    pub attrs: AutoDiffAttrs,
}

#[derive(Clone, Eq, PartialEq, Encodable, Decodable, Debug, HashStable_Generic)]
pub struct AutoDiffAttrs {
    pub mode: DiffMode,
    pub ret_activity: DiffActivity,
    pub input_activity: Vec<DiffActivity>,
}

impl DiffMode {
    pub fn is_rev(&self) -> bool {
        matches!(self, DiffMode::Reverse)
    }
    pub fn is_fwd(&self) -> bool {
        matches!(self, DiffMode::Forward)
    }
}

impl Display for DiffMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DiffMode::Error => write!(f, "Error"),
            DiffMode::Source => write!(f, "Source"),
            DiffMode::Forward => write!(f, "Forward"),
            DiffMode::Reverse => write!(f, "Reverse"),
        }
    }
}

pub fn valid_ret_activity(mode: DiffMode, activity: DiffActivity) -> bool {
    if activity == DiffActivity::None {
        return true;
    }
    match mode {
        DiffMode::Error => false,
        DiffMode::Source => false,
        DiffMode::Forward => {
            activity == DiffActivity::Dual
                || activity == DiffActivity::DualOnly
                || activity == DiffActivity::Const
        }
        DiffMode::Reverse => {
            activity == DiffActivity::Const
                || activity == DiffActivity::Active
                || activity == DiffActivity::ActiveOnly
        }
    }
}

pub fn valid_ty_for_activity(ty: &P<Ty>, activity: DiffActivity) -> bool {
    use DiffActivity::*;
    if matches!(activity, Const) {
        return true;
    }
    if matches!(activity, Dual | DualOnly) {
        return true;
    }
    if matches!(activity, Active | ActiveOnly) {
        return true;
    }
    matches!(ty.kind, TyKind::Ptr(_) | TyKind::Ref(..))
        && matches!(activity, Duplicated | DuplicatedOnly)
}
pub fn valid_input_activity(mode: DiffMode, activity: DiffActivity) -> bool {
    use DiffActivity::*;
    return match mode {
        DiffMode::Error => false,
        DiffMode::Source => false,
        DiffMode::Forward => {
            matches!(activity, Dual | DualOnly | Const)
        }
        DiffMode::Reverse => {
            matches!(
                activity,
                Active | ActiveOnly | Duplicated | DuplicatedOnly | Const
            )
        }
    };
}

impl Display for DiffActivity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffActivity::None => write!(f, "None"),
            DiffActivity::Const => write!(f, "Const"),
            DiffActivity::Active => write!(f, "Active"),
            DiffActivity::ActiveOnly => write!(f, "ActiveOnly"),
            DiffActivity::Dual => write!(f, "Dual"),
            DiffActivity::DualOnly => write!(f, "DualOnly"),
            DiffActivity::Duplicated => write!(f, "Duplicated"),
            DiffActivity::DuplicatedOnly => write!(f, "DuplicatedOnly"),
            DiffActivity::FakeActivitySize => write!(f, "FakeActivitySize"),
        }
    }
}

impl FromStr for DiffMode {
    type Err = ();

    fn from_str(s: &str) -> Result<DiffMode, ()> {
        match s {
            "Error" => Ok(DiffMode::Error),
            "Source" => Ok(DiffMode::Source),
            "Forward" => Ok(DiffMode::Forward),
            "Reverse" => Ok(DiffMode::Reverse),
            _ => Err(()),
        }
    }
}
impl FromStr for DiffActivity {
    type Err = ();

    fn from_str(s: &str) -> Result<DiffActivity, ()> {
        match s {
            "None" => Ok(DiffActivity::None),
            "Active" => Ok(DiffActivity::Active),
            "ActiveOnly" => Ok(DiffActivity::ActiveOnly),
            "Const" => Ok(DiffActivity::Const),
            "Dual" => Ok(DiffActivity::Dual),
            "DualOnly" => Ok(DiffActivity::DualOnly),
            "Duplicated" => Ok(DiffActivity::Duplicated),
            "DuplicatedOnly" => Ok(DiffActivity::DuplicatedOnly),
            _ => Err(()),
        }
    }
}

impl AutoDiffAttrs {
    pub fn has_ret_activity(&self) -> bool {
        self.ret_activity != DiffActivity::None
    }
    pub fn has_active_only_ret(&self) -> bool {
        self.ret_activity == DiffActivity::ActiveOnly
    }

    pub const fn error() -> Self {
        AutoDiffAttrs {
            mode: DiffMode::Error,
            ret_activity: DiffActivity::None,
            input_activity: Vec::new(),
        }
    }
    pub fn source() -> Self {
        AutoDiffAttrs {
            mode: DiffMode::Source,
            ret_activity: DiffActivity::None,
            input_activity: Vec::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.mode != DiffMode::Error
    }

    pub fn is_source(&self) -> bool {
        self.mode == DiffMode::Source
    }
    pub fn apply_autodiff(&self) -> bool {
        !matches!(self.mode, DiffMode::Error | DiffMode::Source)
    }

    pub fn into_item(self, source: String, target: String) -> AutoDiffItem {
        AutoDiffItem {
            source,
            target,
            attrs: self,
        }
    }
}

impl fmt::Display for AutoDiffItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Differentiating {} -> {}", self.source, self.target)?;
        write!(f, " with attributes: {:?}", self.attrs)
    }
}
