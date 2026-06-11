#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use sp_core::{H160, U256};

benchmarks! {
	set_rule {
		let contract = H160([0x11; 20]);
		let selector = [0xaa, 0xbb, 0xcc, 0xdd];
		let min_value = U256::from(10);
	}: _(RawOrigin::Root, contract, selector, min_value, true)
	verify {
		let rule = Rules::<T>::get((contract, selector)).unwrap();
		assert!(rule.enabled);
		assert_eq!(rule.min_value, min_value);
	}

	remove_rule {
		let contract = H160([0x11; 20]);
		let selector = [0xaa, 0xbb, 0xcc, 0xdd];
		Rules::<T>::insert((contract, selector), Rule {
			enabled: true,
			min_value: U256::from(10),
		});
	}: _(RawOrigin::Root, contract, selector)
	verify {
		assert!(!Rules::<T>::contains_key((contract, selector)));
	}

	evaluate {
		let _caller: T::AccountId = whitelisted_caller();
		let contract = H160([0x11; 20]);
		let selector = [0xaa, 0xbb, 0xcc, 0xdd];
		let calldata = [0xaa, 0xbb, 0xcc, 0xdd, 0x01, 0x02];
		Rules::<T>::insert((contract, selector), Rule {
			enabled: true,
			min_value: U256::from(10),
		});
	}: {
		let decision = Pallet::<T>::evaluate(
			contract,
			&calldata,
			U256::from(10),
			T::MaxGaslessGasLimit::get(),
		);
		assert!(decision.is_gasless());
	}

	impl_benchmark_test_suite!(
		Pallet,
		crate::tests::new_test_ext(),
		crate::tests::Test
	);
}
