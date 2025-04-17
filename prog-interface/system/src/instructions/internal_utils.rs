use core::mem::MaybeUninit;

use jiminy_cpi::{account::AccountHandle, AccountPerms};

type CpiAccTup<'a> = (AccountHandle<'a>, AccountPerms);

pub(crate) const fn zip_accounts_perms<'a, const N: usize>(
    accounts: [AccountHandle<'a>; N],
    perms: [AccountPerms; N],
) -> [CpiAccTup<'a>; N] {
    const UNINIT: MaybeUninit<CpiAccTup<'_>> = MaybeUninit::uninit();

    let mut res = [UNINIT; N];
    let mut i = 0;
    while i < N {
        res[i] = MaybeUninit::new((accounts[i], perms[i]));
        i += 1;
    }

    // safety: all elems initialized above
    // TODO: change this to less ugly unsafe code that doesnt potentially
    // copy data once MaybeUninit array methods are stabilized
    unsafe { res.as_ptr().cast::<[CpiAccTup<'a>; N]>().read() }
}

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
