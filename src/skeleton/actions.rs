use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct ActionType: u16 {
        const FOLD = (1 << 0);
        const CALL = (1 << 1);
        const CHECK = (1 << 2);
        const RAISE = (1 << 3);
    }
}

pub enum Action {
    Fold, Call, Check, Raise(i64)
}

impl Action {
    pub fn amount(&self) -> i64 {
        match self {
            Action::Fold => 0,
            Action::Call => 0,
            Action::Check => 0,
            Action::Raise(amt) => *amt
        }
    }
}
