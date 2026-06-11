use alloc::vec;
use alloc::vec::Vec;

use hex_literal::hex;

use crate::AccountId;

/// Sudo / admin account derived from `ADMIN_MNEMONIC` (account #0). Pinned in
/// every genesis to keep the address deterministic across builds.
pub fn admin_account() -> AccountId {
	AccountId::from(hex!("d2aE0A2139dC83Cb920e3cd7B9F640922D14b872"))
}

/// Pre-funded dev users derived from the canonical Hardhat mnemonic
/// `test test test test test test test test test test test junk`
/// (HD path `m/44'/60'/0'/0/N`). These are NOT sudo — they are seeded so
/// E2E suites and Hardhat-derived wallets have spendable balances. We fund
/// all 10 standard Hardhat indices so E2E specs can use signers #5..#9 as
/// clean stashes (the first 4 are already locked as genesis validators on
/// the impetus NPoS spec).
pub fn mnemonic_accounts() -> Vec<AccountId> {
	vec![
		AccountId::from(hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266")), // #0
		AccountId::from(hex!("70997970C51812dc3A010C7d01b50e0d17dc79C8")), // #1
		AccountId::from(hex!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC")), // #2
		AccountId::from(hex!("90F79bf6EB2c4f870365E785982E1f101E93b906")), // #3
		AccountId::from(hex!("15d34AAf54267DB7D7c367839AAf71A00a2C6A65")), // #4
		AccountId::from(hex!("9965507D1a55bcC2695C58ba16FB37d819B0A4dc")), // #5
		AccountId::from(hex!("976EA74026E726554dB657fA54763abd0C3a0aa9")), // #6
		AccountId::from(hex!("14dC79964da2C08b23698B3D3cc7Ca32193d9955")), // #7
		AccountId::from(hex!("23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f")), // #8
		AccountId::from(hex!("a0Ee7A142d267C1f36714E4a8F75612F20a79720")), // #9
	]
}

/// Endowed accounts at genesis: admin first, followed by Hardhat dev users.
pub fn endowed_accounts() -> Vec<AccountId> {
	let mut accounts = vec![admin_account()];
	accounts.extend(mnemonic_accounts());
	accounts
}
