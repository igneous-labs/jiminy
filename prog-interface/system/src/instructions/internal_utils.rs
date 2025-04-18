use jiminy_cpi::AccountPerms;

pub(crate) const fn signer_writable_to_perms<const N: usize>(
    is_signer: [bool; N],
    is_writable: [bool; N],
) -> [AccountPerms; N] {
    let mut res = [AccountPerms {
        is_signer: false,
        is_writable: false,
    }; N];
    let mut i = 0;
    while i < N {
        res[i] = AccountPerms {
            is_signer: is_signer[i],
            is_writable: is_writable[i],
        };
        i += 1;
    }
    res
}
