use jiminy_account::AccountHandle;

/// A CPI instruction
#[derive(Debug, Clone, Copy)]
pub struct Instr<'account, 'data, I> {
    pub prog: AccountHandle<'account>,
    pub accounts: I,
    pub data: &'data [u8],
}
